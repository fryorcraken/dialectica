import QtQuick
import QtTest
import "../src/qml"

// The Stoa list and the join preview, driven through a fake bridge.
//
// What is under test is the screens' own state machines, not a re-implementation
// of them — the shape tst_feed_states.qml established, and the reason for it is
// that a test asserting against its own copy of the logic passes whatever the
// screen does.
//
// Three things this file is written to catch, because each has a plausible bug
// that leaves every other assertion green:
//
//   1. **Empty membership rendered as a failed read, or the reverse.** A user
//      told they belong to nothing will re-join Stoas they are already in.
//   2. **A join that happened without the user asking.** The fake bridge records
//      every method it was called with, so "no join call has been made" is an
//      assertion about a log rather than about a screen's appearance.
//   3. **A share affordance offered for a Stoa whose record the view has not
//      got.** What it would produce is a string that fails to verify on somebody
//      else's machine, as a refusal they cannot explain.
//
// ---- the defect family this file kept producing --------------------------
//
// **Assert the property the user is affected by, not the value feeding it.**
// Five instances have now been found in this one file, four of them by someone
// other than its author, and every one left the whole suite green:
//
//   - a title's markup safety asserted via `Text.text`, which is the SOURCE
//     string and is unchanged by `textFormat` — passed against StyledText;
//   - a share affordance's absence asserted via `canShare()`, the computation
//     feeding the `visible:` binding rather than the binding;
//   - a joined panel's presence asserted via `joinState`, the string feeding it;
//   - a feed's absence asserted via `screenShown`, a derived string;
//   - and the subtlest, found here by mutation: the per-Stoa-identity absence
//     asserted over a FRESH PREVIEW, whose body says nothing whatever about
//     what joining does. Planting "generates you an identity for it alone" in
//     the joined panel — the text a user actually reads — left that assertion
//     PASSING, because the panel was not rendered in the state being scanned.
//
// The last one generalises the rule: an absence assertion is only as strong as
// its corpus, and a corpus with no candidate in it proves nothing. So every
// absence assertion below states what it IS scanning before saying what is not
// there, and `test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus`
// pins that corpus so it cannot silently shrink.
//
// One reason it can shrink is already scheduled: the APPARATUS column is
// annotation explaining the design, not interface, and a separate piece is
// removing it from the shipped view. It is 747 of the 1371 characters a join
// screen renders — measured, not estimated — so a whole-screen scan is more
// than half margin note. `bodyText` exists to scan only what ships. Verified by
// running this file against a hidden apparatus column: all tests pass, because
// none of them depends on annotation.
//
// ---- the second family: a literal where a meaning was required -------------
//
// Distinct from the one above and worth keeping separate, because the fix is
// different. There the assertion read the wrong VALUE; here it reads the right
// value and asks the wrong QUESTION — matching a phrase where the requirement
// is about what a sentence claims. Such a test fails on an innocent reword and
// passes on a fluent lie, which is the worst pair of properties available.
//
//   - three refusal messages asserted to be three DIFFERENT strings. A tester
//     reworded one to actively misinform and both distinguishability tests
//     stayed green: misinforming strings are still distinct strings.
//   - the malformed-paste refusal blocking three exact phrasings. A reviewer
//     measured two equivalents that passed — "could not be confirmed against
//     its address", "failed the check against the address it names" — each
//     telling a user who truncated their own paste to blame their sender.
//   - the address note pinned by three required substrings. A note carrying all
//     three and reading "…which confirms this is the Stoa you were sent"
//     passed, asserting the single thing the spec says the copy may never claim.
//
// So those assertions now test STRUCTURE — a verb of checking together with the
// address as its object; an explicit denial of provenance — rather than
// vocabulary. And two absence assertions that used bare `indexOf` gained word
// boundaries, because `members` matches `membership` and the honest copy
// "Your membership was read without error" is one screen-state change from
// making the count assertion fail for a reason that is not a defect.
//
// Both fixes were verified in BOTH directions, which is the part that is easy
// to skip: a regex that never matches is indistinguishable from a repaired
// assertion until you plant the thing it must still catch.
TestCase {
    id: spec
    name: "StoaScreens"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    // A bridge that answers each method from a map AND records every call.
    //
    // The recording half is not incidental — it is the only way to assert that a
    // call was NOT made. A screen that renders a preview without joining and a
    // screen that joins silently look identical from the outside; the difference
    // is entirely in what reached the bridge.
    function bridgeFor(replies) {
        return {
            calls: [],
            callModule: function (module, method, args) {
                this.calls.push({ method: method, args: args })
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    function callsTo(bridge, method) {
        var n = 0
        for (var i = 0; i < bridge.calls.length; i++)
            if (bridge.calls[i].method === method)
                n++
        return n
    }

    function lastArgsTo(bridge, method) {
        for (var i = bridge.calls.length - 1; i >= 0; i--)
            if (bridge.calls[i].method === method)
                return bridge.calls[i].args
        return null
    }

    // Every descendant carrying `name`, visible or not.
    //
    // Distinct from `visibleNamed` on purpose. Absence assertions must use the
    // visible form — that is the property a user is affected by. But a test that
    // needs to REACH a screen in order to drive it needs it whether or not it is
    // currently on screen: `Main.qml` mounts all three screens and toggles
    // `visible`, so the join screen exists from startup and is hidden until a
    // reference is previewed. Using the visible walker to find it returned
    // nothing and produced "Cannot call method 'join' of undefined" — a test
    // failing for a reason unrelated to the defect it was written for.
    function namedAnywhere(item, name) {
        var found = []
        function walk(node) {
            if (!node)
                return
            if (node.objectName === name)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i])
        }
        walk(item)
        return found
    }

    // Every visible descendant of `item` carrying `name`, at any depth.
    //
    // `visible` is checked on each ancestor rather than trusting the element's
    // own flag: QML's `visible` is not inherited into the property, so a button
    // inside a hidden panel still reports `visible: true` for itself. An absence
    // assertion that missed that would pass on a screen showing the affordance.
    function visibleNamed(item, name) {
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (node.objectName === name && here)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    // Collect the `text` of every visible Text-ish descendant, so a claim about
    // what a screen does NOT say can be checked against everything it does say.
    function visibleText(item) {
        var out = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && typeof node.text === "string" && node.text !== "")
                out.push(node.text)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return out.join("\n")
    }

    // Every visible element a user can type into.
    //
    // **Keyed on behaviour, not on a name or a placeholder.** A walker looking
    // for `objectName: "creatorKeyField"` pins one spelling of the defect; the
    // next one is called `identityPicker`. And a walker keyed on
    // `placeholderText` finds nothing even on the clean tree — the shipped title
    // field is a bare `TextInput` without one, which is the trap the spec-test
    // reviewer flagged after hitting it.
    //
    // These three properties are what `TextInput` and `TextEdit` expose and a
    // `Text` does not, so the set is "things that accept typing" rather than
    // "things that look like a field". `readOnly` is checked because a read-only
    // TextEdit is a display element, not somewhere a user enters a key — the
    // ClipboardSink is exactly that and must not be counted.
    function editableInputs(item) {
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && node.echoMode !== undefined
                     && node.cursorPosition !== undefined
                     && node.readOnly === false)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    // What those inputs currently hold, for a failure message that names which
    // field is the unexpected one rather than only how many there were.
    function describeInputs(inputs) {
        var out = []
        for (var i = 0; i < inputs.length; i++)
            out.push("<" + String(inputs[i].text) + ">")
        return out.join(" ")
    }

    // Every visible element that RENDERS TEXT derived from `address`.
    //
    // Keyed on the rendered text rather than on an `address` property, for two
    // reasons. A hand-rolled elision written as a bare `Text` has no `address`
    // property at all, and would simply not be found — an absence that reads as
    // "nothing renders the address" rather than as the defect it is. And the
    // `Identicon` beside every address DOES carry `address`, so a property-keyed
    // walker returns two elements where one renders characters.
    //
    // So: anything whose text contains the address's first eight characters,
    // which both the right form and every wrong one must render to be an
    // abbreviation of it at all.
    function addressElementsFor(item, address) {
        var head = address.slice(0, 8)
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && typeof node.text === "string" && node.text.indexOf(head) >= 0)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    // The text of the apparatus column alone.
    //
    // **The apparatus is annotation explaining the design, not interface**, and
    // a separate piece is removing it from the shipped view. That matters to
    // every absence assertion in this file: `visibleText` walks the whole
    // ScreenFrame, so more than half of what those assertions scan on the join
    // screen is margin note rather than anything a user acts on. Measured, not
    // estimated — 747 of 1371 characters on a rendered preview.
    //
    // An absence assertion whose corpus shrinks under it does not fail; it
    // quietly starts proving less while its name goes on claiming the same
    // thing. So the two assertions that need a corpus check use this to state
    // what they are scanning, and `bodyText` below to scan only what ships.
    // Note it walks each note through `visibleText`, which checks the note's own
    // ancestors — so a column hidden by its container yields "" here, and
    // `bodyText` then correctly reports the whole screen as body. That is the
    // behaviour the apparatus's removal needs, and it was got wrong first time:
    // an earlier version returned the notes' text whether or not anything
    // rendered them, which made a hidden column look like a column still there.
    function apparatusText(screen) {
        var out = []
        var app = screen.apparatus
        if (app === undefined || app === null)
            return ""
        for (var i = 0; i < app.length; i++) {
            var t = visibleText(app[i])
            if (t !== "")
                out.push(t)
        }
        return out.join("\n")
    }

    // Everything the screen renders EXCEPT the apparatus column — that is, the
    // part that survives the apparatus's removal and the part a user reads.
    //
    // Derived by subtraction rather than by walking a named child, deliberately:
    // `ScreenFrame` exposes its body as a default property alias with no
    // objectName to find, and a lookup keyed on internal structure would break
    // silently when that structure changes. Subtraction breaks loudly instead —
    // if the apparatus stops being separable, `bodyText` returns the whole
    // screen and the corpus assertions below catch it.
    function bodyText(screen) {
        var whole = visibleText(screen)
        var app = apparatusText(screen)
        if (app === "")
            return whole
        var at = whole.indexOf(app)
        return at < 0 ? whole : (whole.slice(0, at) + whole.slice(at + app.length))
    }

    Component { id: listComponent; StoaListScreen {} }
    Component { id: joinComponent; JoinScreen {} }
    Component { id: sinkComponent; ClipboardSink {} }

    function makeList(replies, props) {
        Core.bridge = bridgeFor(replies)
        return listComponent.createObject(null, props === undefined ? {} : props)
    }

    function makeJoin(replies, props) {
        Core.bridge = bridgeFor(replies)
        return joinComponent.createObject(null, props)
    }

    // ---- the three read states ------------------------------------------

    function test_an_empty_membership_is_the_ok_state_and_reports_no_failure() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })

        compare(screen.readState, "ok",
                "a membership that answered with nothing is a SUCCESS, not a failure")
        compare(screen.rows.length, 0)
        compare(screen.failure, "", "an empty read has nothing to report")
        screen.destroy()
    }

    function test_an_unreadable_membership_is_a_failure_carrying_the_cores_words() {
        var screen = makeList({ "list_stoas": '{"error":"the membership store at /x/membership.sqlite is locked"}' })

        compare(screen.readState, "failed")
        compare(screen.rows.length, 0, "a failure must not leave rows behind")
        verify(screen.failure.indexOf("locked") >= 0,
               "core's reason names a fix and must reach the screen unreworded, got: "
               + screen.failure)
        screen.destroy()
    }

    function test_the_empty_state_and_the_failed_state_are_different_states() {
        // Stated directly rather than inferred from the two tests above: the
        // same request against two stores must not leave the screen in one
        // state.
        var empty = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var broken = makeList({ "list_stoas": '{"error":"membership is locked"}' })

        verify(empty.readState !== broken.readState,
               "in-no-Stoas and cannot-read must never reach the same state")
        compare(empty.readState, "ok")
        compare(broken.readState, "failed")

        // And what they RENDER differs, not only what they hold. A pair of
        // states rendered through the same branch would satisfy the compare
        // above and still show the user one screen.
        verify(visibleText(empty) !== visibleText(broken),
               "the two states must not render the same text")
        empty.destroy()
        broken.destroy()
    }

    function test_a_success_without_an_items_array_is_a_named_failure() {
        var noItems = makeList({ "list_stoas": '{"page":0,"hasMore":false}' })
        compare(noItems.readState, "failed",
                "a success with no items array must not render as an empty membership")
        verify(noItems.failure.length > 0, "the failure must name what was wrong")
        noItems.destroy()

        // `items` present but not an array — a shape that passes an
        // `=== undefined` check and then has `.length` read off it.
        var notArray = makeList({ "list_stoas": '{"items":"lots","page":0,"hasMore":false}' })
        compare(notArray.readState, "failed", "items must be an ARRAY, not merely present")
        notArray.destroy()
    }

    function test_an_unreachable_core_is_a_failure_rather_than_an_empty_list() {
        Core.bridge = null
        var screen = listComponent.createObject(null, {})
        compare(screen.readState, "failed")
        verify(screen.failure.toLowerCase().indexOf("not reachable") >= 0,
               "the screen must say the core is unreachable, got: " + screen.failure)
        screen.destroy()
    }

    function test_a_reply_that_is_not_json_is_a_failure_rather_than_a_value() {
        var screen = makeList({ "list_stoas": '<html>gateway timeout</html>' })
        compare(screen.readState, "failed")
        compare(screen.rows.length, 0, "no Stoa may be obtained from a non-JSON reply")
        screen.destroy()
    }

    // ---- what a row carries ---------------------------------------------

    function test_a_row_carries_the_address_as_well_as_the_title() {
        var addr = "7f3a91c4" + "00".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Nym Research"}],'
                        + '"page":0,"hasMore":false}'
        })

        compare(screen.readState, "ok")
        var shown = visibleText(screen)
        verify(shown.indexOf("Nym Research") >= 0, "the founding title must be rendered")
        // The abbreviation is AddressLabel's, so the head and tail are what is on
        // screen — the assertion is that the ADDRESS is there in some form, not
        // that a second elision was written.
        verify(shown.indexOf("7f3a91c4") >= 0,
               "the row must carry the address, not the title alone: " + shown)
        screen.destroy()
    }

    function test_two_stoas_with_the_same_title_render_differently() {
        var a = "7f3a91c4" + "11".repeat(28)
        var b = "b02d5e77" + "22".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":['
                        + '{"stoa":"' + a + '","foundingTitle":"Nym Research"},'
                        + '{"stoa":"' + b + '","foundingTitle":"Nym Research"}'
                        + '],"page":0,"hasMore":false}'
        })

        compare(screen.rows.length, 2, "both rows must be rendered")
        var shown = visibleText(screen)
        // The prefixes are hardcoded rather than derived from the fixture: a
        // check that merely asserted "the two rows differ" would pass on two
        // rows differing in a whitespace character.
        verify(shown.indexOf("7f3a91c4") >= 0, "the first address must be on screen")
        verify(shown.indexOf("b02d5e77") >= 0, "the second address must be on screen")
        screen.destroy()
    }

    function test_an_empty_founding_title_still_gets_a_row_with_its_address() {
        var addr = "1ce0aa38" + "33".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":""}],'
                        + '"page":0,"hasMore":false}'
        })

        compare(screen.readState, "ok")
        compare(screen.rows.length, 1, "an empty title is legal and must not be omitted")
        var shown = visibleText(screen)
        verify(shown.indexOf("1ce0aa38") >= 0, "the row must carry its address")
        // No substitute title. "Untitled" is a title no peer agrees on, and a
        // row showing one would be asserting something the record does not say.
        verify(shown.toLowerCase().indexOf("untitled") < 0,
               "no substitute title may stand in for an empty one")
        verify(shown.indexOf("(no title)") < 0)
        screen.destroy()
    }

    function test_a_title_containing_markup_is_rendered_as_literal_characters() {
        var addr = "aa".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '",'
                        + '"foundingTitle":"<b>bold</b> &amp; <img src=x>"}],'
                        + '"page":0,"hasMore":false}'
        })

        // **The element's `textFormat`, not its `text`.** An earlier version of
        // this test asserted only that `text` contained "<b>bold</b>", and it
        // PASSED against an implementation switched to `Text.StyledText` —
        // proved by mutation. `text` is the SOURCE string and is unchanged by
        // the format, so a test reading it cannot see the rendering at all,
        // which is precisely the property under test. The check has to be on the
        // format, and the source string is asserted alongside it so that a
        // renamed or vanished element fails rather than trivially satisfying it.
        var titles = spec.titleElementsFor(screen, "<b>bold</b> &amp; <img src=x>")
        compare(titles.length, 1, "the title element must be found by its content")
        compare(titles[0].textFormat, Text.PlainText,
                "a peer-supplied title must never reach a markup-interpreting path")
        // AutoText is the trap specifically: it SNIFFS its input and switches to
        // rich text when the string looks like markup, so an element left on the
        // default renders this title as markup while a `text` assertion passes.
        verify(titles[0].textFormat !== Text.AutoText)
        verify(titles[0].textFormat !== Text.StyledText)
        verify(titles[0].textFormat !== Text.RichText)
        screen.destroy()
    }

    // Every visible descendant whose `text` is exactly `value` and which has a
    // `textFormat` — the Text elements carrying peer-supplied content.
    function titleElementsFor(item, value) {
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && node.text === value && node.textFormat !== undefined)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    function test_no_row_renders_a_count_of_held_posts_or_anything_global() {
        // A thread listing is answered too, so a screen tempted to render some
        // other call's page length in the row's margin has one available.
        var addr = "cc".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Transport Notes"}],'
                        + '"page":0,"hasMore":false}',
            "list_threads": '{"items":[{"thread":"t1"},{"thread":"t2"},{"thread":"t3"}],'
                          + '"page":0,"hasMore":true}'
        })

        var shown = visibleText(screen)
        verify(shown.indexOf("posts received here") < 0,
               "no per-row held-post count is available and none may be substituted")
        verify(shown.indexOf("nothing received yet") < 0,
               "that phrase is a claim about a count nothing computed")
        // Word-boundary, not a bare substring. `indexOf("members")` also matches
        // `membership`, and **"Your membership was read without error" is
        // deliberate honest copy** in the list's empty state
        // (StoaListScreen.qml) — the sentence that stops a user re-joining Stoas
        // they are already in. The bare form passes here only because this
        // fixture puts a row on screen, which hides that panel; add a row to the
        // empty-state fixture, or move that sentence into a row, and the test
        // fails for a reason that is not a defect.
        //
        // The two words mean opposite things: `members` is a global count no
        // peer can observe and the spec forbids permanently; `membership` is
        // what this machine recorded and is the honest thing to say.
        verify(!/\bmembers\b/.test(shown) && shown.indexOf("peers reachable") < 0,
               "no global count may appear: " + shown)
        // The specific substitution the spec forbids by name: another call's
        // page length rendered as though it were a total.
        verify(shown.indexOf("3 posts") < 0 && shown.indexOf("3 POSTS") < 0,
               "a page length from another call must not appear in a row")
        screen.destroy()
    }

    // ---- sharing ---------------------------------------------------------

    function test_a_share_is_offered_only_where_the_view_holds_the_record() {
        var withRecord = "aa".repeat(32)
        var without = "bb".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":['
                        + '{"stoa":"' + withRecord + '","foundingTitle":"Held"},'
                        + '{"stoa":"' + without + '","foundingTitle":"Not held"}'
                        + '],"page":0,"hasMore":false}'
        })
        // One of the two rows has a record; the other does not. A single fixture
        // for both halves is deliberate: **an absence assertion alone proves
        // nothing**, because a renamed or deleted button makes it pass. Here the
        // POSITIVE half uses the same lookup, so a rename breaks this test rather
        // than silently satisfying it.
        var held = {}
        held[withRecord] = "00ff00ff"
        screen.genesisByStoa = held
        screen.reload()

        compare(screen.canShare(withRecord), true,
                "a Stoa whose record the view holds can be shared")
        compare(screen.canShare(without), false,
                "a Stoa whose record the view has NOT got must offer no share")

        // And nothing is produced for it, not merely nothing offered.
        compare(StoaReference.shareText(without, screen.genesisFor(without)), "",
                "nothing carrying the address without a record may be produced")
        screen.destroy()
    }

    function test_a_share_carries_both_halves_and_the_address_in_full() {
        var addr = "b02d5e77a41c6b9013c6a9408ff4af235d7e1b06c92a84f13be057dc6104a8bf"
        var genesis = "0102030405060708"
        var text = StoaReference.shareText(addr, genesis)

        verify(text.indexOf(addr) >= 0,
               "the whole address must be present, unabbreviated: " + text)
        verify(text.indexOf(genesis) >= 0, "the genesis record must be present")
        // Unabbreviated, stated as the relation rather than as a length: an
        // ellipsis anywhere means an abbreviation reached the shareable thing.
        verify(text.indexOf("…") < 0, "no elision may appear in a shareable reference")
    }

    function test_what_a_share_produces_is_what_a_paste_accepts() {
        // The round trip, which is the whole reason the two live in one file. A
        // share whose output the paste field cannot read produces a string whose
        // recipient can do nothing with it.
        var addr = "b02d5e77" + "44".repeat(28)
        var genesis = "deadbeefcafe"
        var parsed = StoaReference.parse(StoaReference.shareText(addr, genesis))

        compare(parsed.ok, true, "a share's own output must parse: ")
        compare(parsed.stoa, addr, "the preview must name the same Stoa")
        compare(parsed.genesis, genesis)
    }

    function test_a_share_string_reaches_the_clipboard_sink_verbatim() {
        var sink = sinkComponent.createObject(null, {})
        var addr = "aa".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Held"}],'
                        + '"page":0,"hasMore":false}'
        }, { clipboard: sink })
        var held = {}
        held[addr] = "00ff"
        screen.genesisByStoa = held
        screen.reload()

        var buttons = visibleNamed(screen, "shareButton")
        compare(buttons.length, 1, "exactly one share affordance for the one held record")
        buttons[0].clicked()

        // **What is asserted is what the sink was ASKED to copy.** Under
        // QT_QPA_PLATFORM=offscreen there is no system clipboard, so no test
        // anywhere can check what actually landed on it — TextEdit.copy() writes
        // nowhere readable. That half is unverified, here and in CI, and is
        // stated rather than papered over.
        compare(sink.lastCopied, StoaReference.shareText(addr, "00ff"))
        verify(sink.lastCopied.indexOf(addr) >= 0)
        verify(sink.lastCopied.indexOf("00ff") >= 0)
        screen.destroy()
        sink.destroy()
    }

    // ---- pasting ---------------------------------------------------------

    function test_text_that_is_not_a_stoa_reference_is_refused_before_any_call() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        var before = bridge.calls.length

        screen.pasted = "stoa:b02d5e77a41c6b9013c6a9408ff4af23"
        screen.preview()

        verify(screen.pasteFailure.length > 0, "the screen must say what was pasted is not a reference")
        compare(spec.callsTo(bridge, "join_stoa"), 0,
                "a malformed paste must reach no join call")
        compare(bridge.calls.length, before,
                "a malformed paste must reach NO call at all")
        screen.destroy()
    }

    function test_an_address_with_no_record_is_named_as_the_missing_half() {
        var parsed = StoaReference.parse('{"stoa":"' + "aa".repeat(32) + '"}')
        compare(parsed.ok, false)
        verify(parsed.reason.toLowerCase().indexOf("record") >= 0,
               "the refusal must name the half that is missing: " + parsed.reason)
    }

    function test_a_well_formed_reference_previews_and_joins_nothing() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        var seen = { stoa: "", genesis: "", count: 0 }
        screen.previewRequested.connect(function (stoa, genesis) {
            seen.stoa = stoa
            seen.genesis = genesis
            seen.count++
        })

        var addr = "b02d5e77" + "55".repeat(28)
        screen.pasted = StoaReference.shareText(addr, "00ff")
        screen.preview()

        compare(seen.count, 1, "the preview must be requested")
        compare(seen.stoa, addr)
        compare(screen.pasteFailure, "", "a well-formed reference is not a paste failure")
        compare(spec.callsTo(bridge, "join_stoa"), 0,
                "acting on an address must PREVIEW and never join")
        screen.destroy()
    }

    function test_a_display_prefix_is_stripped_before_anything_is_sent() {
        // `stoa:` is a reading aid the design bundle uses. The core writes and
        // accepts bare hex, so a prefix that survived into join_stoa would be a
        // hash verifying against nothing — surfacing as a VERIFICATION failure
        // rather than as the malformed paste it actually is, which is the one
        // confusion the three outcomes exist to prevent.
        var addr = "b02d5e77" + "66".repeat(28)
        var parsed = StoaReference.parse('{"stoa":"stoa:' + addr + '","genesis":"00ff"}')
        compare(parsed.ok, true)
        compare(parsed.stoa, addr, "the prefix must not reach the core")
        verify(StoaReference.shareText(addr, "00ff").indexOf("stoa:") < 0,
               "and must never be added on the way out")
    }

    function test_a_repeated_display_prefix_is_stripped_rather_than_forwarded() {
        // **A single-pass strip is the defect, and the test above could not see
        // it** because it only ever supplied one prefix. A doubled `stoa:`
        // survived, reached `join_stoa` as part of the address, and came back as
        // "the genesis record does not hash to this address" — turning a
        // malformed paste into a VERIFICATION ACCUSATION and pointing the user
        // at their sender instead of at their own paste.
        //
        // That is the exact confusion the three paste outcomes are designed to
        // keep apart, and both design.md and StoaReference's own source comment
        // assert it cannot happen. The invariant the comment claimed was not the
        // one the code enforced.
        var addr = "b02d5e77" + "88".repeat(28)
        var parsed = StoaReference.parse('{"stoa":"stoa:stoa:' + addr + '","genesis":"00ff"}')
        compare(parsed.ok, true)
        compare(parsed.stoa, addr, "every prefix must be stripped, not just one")

        // Whitespace between them too — a paste that picked up a stray space is
        // the same user error with the same right answer.
        compare(StoaReference.parse('{"stoa":"stoa: stoa:' + addr + '","genesis":"00ff"}').stoa,
                addr)
        compare(StoaReference.parse('{"stoa":"  stoa:stoa:stoa:' + addr + '  ","genesis":"00ff"}').stoa,
                addr, "three, with surrounding whitespace")

        // And the end-to-end consequence: nothing carrying a prefix reaches the
        // core, so no verification refusal can be provoked by a paste artefact.
        Core.bridge = bridgeFor({ "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"t"}' })
        var bridge = Core.bridge
        Core.joinStoa(parsed.stoa, parsed.genesis)
        var sent = String(spec.lastArgsTo(bridge, "join_stoa"))
        verify(sent.indexOf("stoa:") < 0,
               "no prefix may reach join_stoa: " + sent)
    }

    // ---- the join preview ------------------------------------------------

    function test_the_preview_makes_no_join_call_until_the_user_acts() {
        var addr = "b02d5e77" + "77".repeat(28)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff", foundingTitle: "Nym Research" })
        var bridge = Core.bridge

        // Rendered, and nothing called. This is the assertion the recording
        // bridge exists for: a screen that previews and a screen that joins
        // silently are indistinguishable from the outside.
        compare(screen.joinState, "previewing")
        compare(bridge.calls.length, 0,
                "rendering a preview must make NO core call whatever")

        screen.join()

        compare(spec.callsTo(bridge, "join_stoa"), 1,
                "the join call is made when, and only when, the user acts")
        compare(screen.joinState, "joined")
        screen.destroy()
    }

    // ---- a second preview inherits nothing from the first -----------------
    //
    // **These drive `Main.qml`, not a fresh `JoinScreen`, and that is the whole
    // point of them.** Every other join-state test in this file constructs its
    // own screen — and `Main.qml` ships ONE reused instance, which is the only
    // configuration a user ever meets. So the suite could be entirely green
    // while a hostile reference rendered under a "Joined." panel it never
    // earned, and it was: 95 of 95 passed with that defect present.
    //
    // That is the corpus lesson at integration scale. A test exercising a
    // component in a shape the app does not use is testing something the app
    // does not do.

    function test_a_second_preview_does_not_inherit_the_first_joined_state() {
        var a = "aaaaaaaa" + "11".repeat(28)
        var b = "bbbbbbbb" + "22".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "join_stoa": '{"stoa":"' + a + '","foundingTitle":"Nym Research","policy":"open"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        // Preview A and join it, the legitimate half of the sequence.
        view.previewing = { stoa: a, genesis: "00ff" }
        join.join()
        compare(join.joinState, "joined", "A was genuinely joined")
        compare(spec.callsTo(bridge, "join_stoa"), 1)

        // Now the attacker's reference, with NO Cancel in between — an ordinary
        // two-paste sequence, not a contrived one.
        view.previewing = { stoa: b, genesis: "00ff" }

        compare(join.stoaAddress, b, "the attacker's address is what is on screen")
        compare(spec.callsTo(bridge, "join_stoa"), 1,
                "no join call was made for B")
        compare(join.joinState, "previewing",
                "so B must NOT read as joined — a join is reported from the "
                + "core's reply, never inherited from another Stoa's")

        // The user-visible consequence, not only the state string feeding it.
        compare(spec.visibleNamed(join, "joinedPanel").length, 0,
                "no joined panel for a Stoa the core never saw")
        compare(spec.visibleNamed(join, "joinButton").length, 1,
                "and the join affordance must be back, or the user cannot act")
        var body = spec.bodyText(join)
        verify(body.indexOf("started collecting") < 0,
               "nothing may claim this machine began collecting B's records: " + body)
        view.destroy()
    }

    function test_a_second_preview_does_not_inherit_the_first_founding_title() {
        var a = "aaaaaaaa" + "33".repeat(28)
        var b = "bbbbbbbb" + "44".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "join_stoa": '{"stoa":"' + a + '","foundingTitle":"Nym Research","policy":"open"}'
        })
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        view.previewing = { stoa: a, genesis: "00ff" }
        join.join()
        compare(join.foundingTitle, "Nym Research", "A's title came from A's reply")

        view.previewing = { stoa: b, genesis: "00ff" }

        // **Separable from the state carryover, and it must be asserted
        // separately.** Clearing `joinState` alone would still leave A's title
        // captioned "FIXED FOREVER" above B's address — a trusted name lent to
        // an untrusted address, which is precisely the impersonation the
        // lookalike requirement exists to expose, delivered by the view itself.
        // The lookalike panel cannot catch it: the two titles being compared
        // are the same string from the same source.
        compare(join.foundingTitle, "",
                "B's title is unknown until B's record is decoded, so nothing "
                + "may be rendered as B's founding title")
        var body = spec.bodyText(join)
        verify(body.indexOf("Nym Research") < 0,
               "A's title must not decorate B's address: " + body)
        view.destroy()
    }

    function test_a_second_preview_does_not_inherit_the_first_refusal() {
        var a = "aaaaaaaa" + "55".repeat(28)
        var b = "bbbbbbbb" + "66".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "join_stoa": '{"error":"the genesis record does not hash to this address"}'
        })
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        view.previewing = { stoa: a, genesis: "00ff" }
        join.join()
        compare(join.joinState, "failed", "A was refused by the core")

        view.previewing = { stoa: b, genesis: "00ff" }

        // The mirror of the joined case and just as wrong: a user reads "the
        // record you were sent is wrong" about a reference the core has not
        // seen. Listed separately by the reviewer because fixing either of the
        // two above does not fix this one.
        compare(join.joinState, "previewing")
        compare(join.failure, "", "A's refusal says nothing about B")
        compare(spec.visibleNamed(join, "joinFailurePanel").length, 0,
                "no refusal may be attached to an address never submitted")
        view.destroy()
    }

    function test_an_outcome_survives_re_previewing_the_very_same_reference() {
        // **The other side of the rule, and I got this one wrong first.**
        //
        // My initial version of this test asserted that re-previewing the SAME
        // reference after joining it must read `previewing` again — that Cancel
        // should wipe the outcome. It failed against the fix, and the fix was
        // right: the core genuinely answered for this exact (stoa, genesis)
        // pair, so reporting it is reporting a fact, not inheriting another
        // Stoa's. Making the screen forget a true answer would have been a
        // second defect dressed as caution, and would have cost a real join call
        // every time a user glanced back at a Stoa they already hold.
        //
        // The invariant is not "clear on navigation" — that is the reset shape
        // the fix deliberately avoided. It is "an outcome describes exactly the
        // reference it was returned for", which is true here and false for the
        // three tests above. Recorded so nobody later "fixes" this into a wipe.
        var a = "aaaaaaaa" + "77".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "join_stoa": '{"stoa":"' + a + '","foundingTitle":"Nym Research","policy":"open"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        view.previewing = { stoa: a, genesis: "00ff" }
        join.join()
        join.cancelled()
        compare(view.previewing, null)

        view.previewing = { stoa: a, genesis: "00ff" }
        compare(join.joinState, "joined",
                "the core answered for THIS reference, so the answer stands")
        compare(join.foundingTitle, "Nym Research",
                "and the title it returned belongs to this Stoa")
        compare(spec.callsTo(bridge, "join_stoa"), 1,
                "and no second call was needed to say so")

        // A DIFFERENT genesis for the same address is a different reference, and
        // must not inherit: the address alone does not identify what was
        // verified, since the pair is what the core checked.
        view.previewing = { stoa: a, genesis: "ffff" }
        compare(join.joinState, "previewing",
                "a different record against the same address is a different "
                + "reference and carries no outcome")
        compare(join.foundingTitle, "")
        view.destroy()
    }

    function test_the_preview_renders_the_whole_address_not_an_abbreviation() {
        var addr = "b02d5e77a41c6b9013c6a9408ff4af235d7e1b06c92a84f13be057dc6104a8bf"
        var screen = makeJoin({}, { stoaAddress: addr, stoaGenesis: "00ff" })

        var labels = spec.visibleNamed(screen, "previewAddress")
        compare(labels.length, 1)
        compare(labels[0].full, true, "a decision is made here; the full form is required")
        compare(labels[0].text, addr,
                "the whole address must be rendered rather than an abbreviation")
        verify(labels[0].text.indexOf("…") < 0, "no elision on this screen")
        screen.destroy()
    }

    function test_the_founding_title_is_labelled_as_founding() {
        var screen = makeJoin({}, {
            stoaAddress: "aa".repeat(32),
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research"
        })

        var shown = spec.visibleText(screen)
        verify(shown.indexOf("Nym Research") >= 0, "the founding title must be rendered")
        verify(shown.indexOf("FOUNDING TITLE") >= 0,
               "and identified as the FOUNDING value: " + shown)

        // Same exposure as a list row: this title is peer-supplied and freely
        // chosen. Asserted on the FORMAT rather than on the text, for the reason
        // spelled out on the list's markup test — `text` cannot see the
        // rendering, which is the property in question.
        var titles = spec.visibleNamed(screen, "foundingTitleText")
        compare(titles.length, 1)
        compare(titles[0].textFormat, Text.PlainText,
                "the founding title must never reach a markup-interpreting path")
        screen.destroy()
    }

    function test_no_current_title_is_rendered_while_nothing_resolves_one() {
        var screen = makeJoin({}, {
            stoaAddress: "aa".repeat(32),
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research"
        })

        var shown = spec.visibleText(screen)
        verify(shown.indexOf("CURRENT TITLE") < 0,
               "the founding value must not be captioned as a current title: " + shown)
        verify(shown.toLowerCase().indexOf("chosen by a moderator") < 0,
               "nothing may attribute this title to a moderator")
        // Rendered ONCE. Twice under two captions would be the same assertion
        // made in the layout instead of the copy.
        compare(spec.visibleNamed(screen, "currentTitleText").length, 0)
        compare(spec.visibleNamed(screen, "foundingTitleText").length, 1)

        // **The two literals above are not the requirement**, and a reviewer
        // proved it: extending the founding caption to `FOUNDING TITLE — FIXED
        // FOREVER — AND THIS STOA'S PRESENT NAME, AS ITS MODERATOR HAS IT TODAY`
        // carries neither blocked string and asserts both things R6 forbids. All
        // 100 tests passed. Same family as the address note — the assertion read
        // the right value and asked the wrong question.
        //
        // So: the *claim* is refused, however phrased. Two conjunctions, because
        // the requirement forbids two distinct things and either alone is the
        // harm. On a build where metadata resolution is not implemented, copy
        // saying this title is what the Stoa is called NOW asserts that no
        // moderator has renamed it — a fact no peer here has checked and which
        // is false for every Stoa that has been renamed.
        var low = shown.toLowerCase()

        // (a) a present-tense naming word attached to a title.
        verify(!/\b(present|current|now|today|at the moment|as it stands|presently)\b[^.]{0,80}\b(name|title|called|known as)\b/.test(low)
               && !/\b(name|title|called|known as)\b[^.]{0,80}\b(present|current|now|today|at the moment|as it stands|presently)\b/.test(low),
               "nothing may present a title as what the Stoa is called now — "
               + "nothing on this build resolves a current title, so such a "
               + "caption asserts no moderator has renamed it: " + shown)

        // (b) a title attributed to a moderator, in any voice.
        verify(!/\bmoderator\b[^.]{0,80}\b(name|title|called|renamed|has it|chose|chosen)\b/.test(low)
               && !/\b(name|title|called|renamed)\b[^.]{0,80}\bmoderator\b/.test(low),
               "nothing may attribute this title to a moderator: " + shown)
        screen.destroy()
    }

    function test_a_resolved_current_title_fills_that_position_when_one_exists() {
        // The panel arrives WITH the value. This is the forward half of the rule
        // above: the prohibition is on claiming, not on the layout.
        var screen = makeJoin({}, {
            stoaAddress: "aa".repeat(32),
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research",
            currentTitle: "Nym Research Archive"
        })

        var shown = spec.visibleText(screen)
        verify(shown.indexOf("Nym Research Archive") >= 0, "the resolved title must render")
        verify(shown.indexOf("FOUNDING TITLE") >= 0, "and the founding one stay labelled")
        verify(shown.indexOf("CURRENT TITLE") >= 0,
               "each labelled as the value it is, so the reader can tell which was fixed")
        screen.destroy()
    }

    function test_the_explanation_claims_a_hash_match_and_names_the_unverified_rest() {
        var screen = makeJoin({}, { stoaAddress: "aa".repeat(32), stoaGenesis: "00ff" })
        var notes = spec.visibleNamed(screen, "addressNote")
        compare(notes.length, 1)
        var note = notes[0].text

        verify(note.indexOf("this address names") >= 0,
               "what is stated is that the record shown is the one the address names")
        verify(note.toLowerCase().indexOf("unverified") >= 0
               || note.toLowerCase().indexOf("not checked") >= 0,
               "the unverified remainder must be named: " + note)
        // And the overreach the bundle's own string carries is absent: nothing
        // says pasting IS the verification, full stop.
        verify(note.indexOf("is itself the verification") < 0,
               "the copy must not present pasting as THE verification")
        verify(note.toLowerCase().indexOf("registry") >= 0,
               "and must say the address was checked against no registry")
        screen.destroy()
    }

    function test_nothing_on_the_preview_promises_a_per_stoa_identity() {
        var addr = "aa".repeat(32)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff" })

        // **Driven through the JOINED state, and that is the point of the test
        // rather than incidental setup.** An earlier version asserted over a
        // fresh preview, whose card body says nothing whatever about what
        // joining does — the only sentence on that subject lived in the
        // apparatus column, which is annotation and is being removed. So the
        // assertion was scanning a screen that could not plausibly have carried
        // the claim: an absence proved over a corpus with no candidate in it,
        // which is the same "passes for the wrong reason" shape as asserting a
        // binding's input instead of its output.
        //
        // After a join the body DOES speak about what joining did, so this now
        // examines the text where the false promise would actually appear.
        screen.join()
        compare(screen.joinState, "joined", "the state that says what joining did")

        var body = spec.bodyText(screen)
        verify(body.indexOf("started collecting") >= 0,
               "the body must be saying what joining did, or this proves nothing "
               + "about the screen a user reads: " + body)

        var shown = body.toLowerCase()

        // The bundle's apparatus note ends "and generates you an identity for it
        // alone". One key signs in every Stoa in this release, so that sentence
        // promises an unlinkability property the software does not have — the
        // one false claim on these screens that could actually harm somebody.
        verify(shown.indexOf("identity for it alone") < 0,
               "no per-Stoa identity may be promised")
        verify(shown.indexOf("an identity for that stoa") < 0)
        verify(shown.indexOf("identity for that stoa alone") < 0)
        // The claim need not use the bundle's exact words to do the harm, so the
        // word itself is refused anywhere in the body. Nothing this screen
        // legitimately says uses it.
        // Word-boundary, for the same reason as `\bmembers\b` on the list's count
        // assertion. `identical` does not in fact contain `identity` — this one
        // is safe by luck of spelling today — but the words it WOULD catch are
        // `identities`, `identify` and `identifier`, and the lookalike panel is
        // one copy edit from saying "the titles are identical" on screen. A
        // false alarm naming a per-Stoa-identity promise that was never made is
        // how a real one later gets waved through.
        verify(!/\bidentity\b/.test(shown),
               "the body must not raise identity at all, however phrased: " + body)
        verify(shown.indexOf("you moderate") < 0, "no moderator status may be asserted")
        verify(shown.indexOf("membership list") < 0 || shown.indexOf("no membership list") >= 0,
               "membership may be denied but never asserted")
        screen.destroy()
    }

    // The corpus these absence assertions run over, asserted rather than assumed.
    //
    // **An absence assertion cannot tell you its corpus shrank.** The apparatus
    // column is annotation that a separate piece is removing from the shipped
    // view, and more than half the text `visibleText` returns for a join screen
    // comes from it. Without this, that removal would silently narrow two tests
    // above while their names went on claiming the same coverage — so the
    // relationship is pinned here, and it fails loudly whichever way it breaks.
    function test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus() {
        var addr = "aa".repeat(32)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff", foundingTitle: "Nym Research" })
        screen.join()

        var body = spec.bodyText(screen)
        var app = spec.apparatusText(screen)

        // The body carries real, on-topic sentences of its own. If the apparatus
        // were ever the only place a subject was discussed, an absence assertion
        // over the body would be vacuous — which is exactly the defect this
        // pins.
        verify(body.length > 0, "the body must carry text of its own")
        verify(body.indexOf("started collecting") >= 0,
               "what joining did is stated in the BODY, not only in a margin note")
        verify(body.indexOf("Nym Research") >= 0, "as is the founding title")

        // And where an apparatus is rendered at all, the two are disjoint — so
        // `bodyText` is genuinely subtracting it rather than returning the whole
        // screen and being believed.
        //
        // **Guarded on the apparatus being non-empty, and that guard is the
        // point rather than a convenience.** The apparatus is annotation a
        // separate piece is removing; once it is gone `app` is "" and `body` IS
        // the whole screen, correctly. An unguarded "body must be smaller than
        // the screen" would fail on that change while the two behaviour tests it
        // exists to protect kept passing — a guard failing for the one reason
        // that is not a defect. Verified by running this file against a hidden
        // apparatus column: the two behaviour tests pass, and only this
        // assertion had to be taught the difference.
        if (app !== "") {
            verify(body.indexOf(app) < 0,
                   "bodyText must not still contain the apparatus")
            verify(spec.visibleText(screen).length > body.length,
                   "and must be strictly smaller than the whole screen")
        }
        screen.destroy()
    }

    // ---- the two failures, kept apart -------------------------------------

    function test_a_record_that_does_not_verify_is_a_failure_distinct_from_a_bad_paste() {
        var addr = "b02d5e77" + "88".repeat(28)
        var join = makeJoin({
            "join_stoa": '{"error":"the genesis record does not hash to this address; '
                       + 'whoever sent it did not send the record this address names"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff" })
        join.join()

        compare(join.joinState, "failed")
        verify(join.failure.indexOf("does not hash") >= 0,
               "the core's refusal must be rendered: " + join.failure)
        var refusal = spec.visibleText(join)

        // The other failure: text that is not a reference at all.
        var list = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        list.pasted = "not a reference"
        list.preview()
        var malformed = list.pasteFailure

        verify(refusal !== malformed,
               "a record that does not verify and a malformed paste must not read alike")
        verify(refusal.indexOf(malformed) < 0,
               "nor may one contain the other verbatim")
        join.destroy()
        list.destroy()
    }

    function test_the_view_does_not_report_a_pair_as_joinable_before_the_core_answers() {
        // A well-formed pair whose record does not name the address. The view's
        // own check is limited to whether both halves are present — a view that
        // re-derived an address would be a second implementation of the one
        // check this design rests on.
        var addr = "b02d5e77" + "99".repeat(28)
        var parsed = StoaReference.parse(StoaReference.shareText(addr, "00"))
        compare(parsed.ok, true,
                "the view must accept a well-formed pair without judging it")

        var screen = makeJoin({
            "join_stoa": '{"error":"the genesis record does not hash to this address"}'
        }, { stoaAddress: addr, stoaGenesis: "00" })

        compare(screen.joinState, "previewing",
                "nothing is reported as joinable before the core has answered")
        screen.join()
        compare(screen.joinState, "failed",
                "the outcome comes from the core's reply, not from a check in the view")
        screen.destroy()
    }

    function test_a_failed_join_does_not_report_the_stoa_as_joined() {
        var joinedFired = 0
        var screen = makeJoin({
            "join_stoa": '{"error":"the membership store is read-only"}'
        }, { stoaAddress: "aa".repeat(32), stoaGenesis: "00ff" })
        screen.joined.connect(function () { joinedFired++ })

        screen.join()

        compare(screen.joinState, "failed")
        compare(joinedFired, 0, "a failure must not emit the joined signal")
        verify(spec.visibleText(screen).indexOf("read-only") >= 0,
               "the core's failure must be rendered")
        compare(spec.visibleNamed(screen, "joinedPanel").length, 0,
                "and the joined panel must not be on screen")
        screen.destroy()
    }

    function test_joining_a_stoa_already_held_is_success_with_no_warning() {
        var addr = "7f3a91c4" + "aa".repeat(28)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, {
            stoaAddress: addr,
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research",
            // The very same Stoa is already in the held listing — which is the
            // input a screen inferring newness would reach for.
            heldStoas: [{ stoa: addr, foundingTitle: "Nym Research" }]
        })

        screen.join()

        compare(screen.joinState, "joined", "a repeat join is SUCCESS")

        // The BODY, not the whole frame. The apparatus column is annotation
        // being removed from the shipped view, and scanning it would let this
        // assertion quietly narrow when that lands. The corpus is asserted
        // first so the absence below is over text that exists.
        var body = spec.bodyText(screen)
        verify(body.indexOf("Joined.") >= 0,
               "the outcome must be on screen, or the absences prove nothing: " + body)

        var shown = body.toLowerCase()
        verify(shown.indexOf("already") < 0, "no warning about a repeat")
        verify(shown.indexOf("collision") < 0 && shown.indexOf("duplicate") < 0,
               "and no collision to resolve: " + shown)
        // Word-boundary, not a bare substring: a plain `indexOf("again")` fires
        // on "not checked AGAINst any registry", which is legitimate copy in the
        // address note. Caught by this assertion failing on first run, which is
        // the argument for running a new assertion before believing it.
        verify(!/\bagain\b/.test(shown), "nor any nod to this being a second attempt")
        verify(!/\bonce more\b/.test(shown))

        // The user-visible consequence, not the state string that feeds it: the
        // failure panel must not be ON SCREEN. `visibleNamed` checks every
        // ancestor's visibility, so a panel inside a hidden parent counts as
        // absent and a panel bound `visible: true` counts as present.
        compare(spec.visibleNamed(screen, "joinFailurePanel").length, 0,
                "no failure panel may be rendered for a repeat join")
        compare(spec.visibleNamed(screen, "joinedPanel").length, 1,
                "and the joined panel must actually be rendered")
        screen.destroy()
    }

    // ---- the lookalike ----------------------------------------------------

    function test_a_same_title_stoa_already_held_is_shown_beside_the_preview() {
        var previewed = "b02d5e77" + "bb".repeat(28)
        var held = "7f3a91c4" + "cc".repeat(28)
        var screen = makeJoin({}, {
            stoaAddress: previewed,
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research",
            heldStoas: [{ stoa: held, foundingTitle: "Nym Research" }]
        })

        compare(screen.lookalikes.length, 1)
        var shown = spec.visibleText(screen)
        // Both addresses. The titles are identical, so the addresses are the
        // only thing telling the reader these are two Stoas.
        verify(shown.indexOf("b02d5e77") >= 0, "the previewed address must be on screen")
        verify(shown.indexOf("7f3a91c4") >= 0, "the held one's address too")
        verify(shown.toLowerCase().indexOf("duplicate") < 0
               && shown.toLowerCase().indexOf("conflict") < 0,
               "neither is a duplicate or a conflict: " + shown)
        screen.destroy()
    }

    function test_the_same_address_is_not_shown_beside_itself_as_a_second_stoa() {
        var addr = "b02d5e77" + "dd".repeat(28)
        var screen = makeJoin({}, {
            stoaAddress: addr,
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research",
            heldStoas: [{ stoa: addr, foundingTitle: "Nym Research" }]
        })

        compare(screen.lookalikes.length, 0,
                "a Stoa already held at the SAME address is not a second Stoa")
        compare(spec.visibleNamed(screen, "lookalikePanel").length, 0)
        screen.destroy()
    }

    // ---- creation ---------------------------------------------------------

    function test_the_create_affordance_is_present_when_no_key_exists() {
        // Offered whatever the keystore holds. The posting probe takes a Stoa
        // address and there is no Stoa yet, so there is nothing to ask it — a
        // hidden button would be hidden on a guess rather than an answer.
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_capabilities": '{"canPost":false,"reason":"No keystore found."}'
        })

        var buttons = spec.visibleNamed(screen, "createStoaButton")
        compare(buttons.length, 1, "the create affordance must be present and actionable")
        compare(buttons[0].enabled, true)
        screen.destroy()
    }

    function test_a_creation_refused_for_want_of_a_key_renders_the_cores_reason() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "create_stoa": '{"error":"No keystore found. Create one before posting; '
                         + 'the reply box comes back when a reply would actually send."}'
        })
        screen.createTitle = "Transport Notes"
        screen.create()

        compare(screen.createState, "failed")
        compare(screen.created, null, "nothing may be reported as created")
        verify(spec.visibleText(screen).indexOf("No keystore found.") >= 0,
               "the keystore's own reason, unreworded")
        screen.destroy()
    }

    function test_an_empty_title_reaches_the_core_rather_than_being_refused_here() {
        var addr = "ee".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "create_stoa": '{"stoa":"' + addr + '","foundingTitle":"","policy":"open"}'
        })
        var bridge = Core.bridge
        screen.createTitle = ""
        screen.create()

        compare(spec.callsTo(bridge, "create_stoa"), 1,
                "the create call must be MADE — the reply decides, not a check here")
        compare(screen.createState, "created")
        verify(String(spec.lastArgsTo(bridge, "create_stoa")).indexOf('"title":""') >= 0,
               "and the empty title must be what was sent")
        screen.destroy()
    }

    function test_the_created_address_is_rendered() {
        var addr = "b02d5e77a41c6b9013c6a9408ff4af235d7e1b06c92a84f13be057dc6104a8bf"
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "create_stoa": '{"stoa":"' + addr + '","foundingTitle":"Transport Notes","policy":"open"}'
        })
        screen.createTitle = "Transport Notes"
        screen.create()

        var labels = spec.visibleNamed(screen, "createdAddress")
        compare(labels.length, 1, "there is no registry to look a Stoa up in later")
        compare(labels[0].text, addr, "the address the core returned, in full")
        screen.destroy()
    }

    function test_the_same_title_twice_is_one_stoa_reported_twice() {
        var addr = "b02d5e77" + "ff".repeat(28)
        var reply = '{"stoa":"' + addr + '","foundingTitle":"Transport Notes","policy":"open"}'
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "create_stoa": reply
        })
        var bridge = Core.bridge

        screen.createTitle = "Transport Notes"
        screen.create()
        var firstAddress = screen.created.stoa

        screen.create()

        compare(screen.createState, "created", "the second outcome is SUCCESS")
        compare(screen.created.stoa, firstAddress,
                "and names the Stoa the first creation returned")
        var shown = spec.visibleText(screen).toLowerCase()
        verify(shown.indexOf("collision") < 0 && shown.indexOf("duplicate") < 0
               && shown.indexOf("already exists") < 0,
               "nothing may read as a collision: " + shown)

        // The title submitted is the title the user typed, both times. A suffix
        // appended to obtain a different Stoa mints one with a title they did
        // not choose, permanently, at an address nobody can withdraw.
        var sent = String(spec.lastArgsTo(bridge, "create_stoa"))
        verify(sent.indexOf('"title":"Transport Notes"') >= 0,
               "nothing may be appended to the user's title: " + sent)
        screen.destroy()
    }

    // ---- what the view holds of its own -----------------------------------

    function test_the_view_supplies_no_stoa_of_its_own_before_one_is_chosen() {
        // The top-level view used to carry stoaAddress/stoaTitle/stoaGenesis as
        // developer-filled properties. A second source for the one value these
        // screens exist to supply is a build that ships a hardcoded Stoa.
        Core.bridge = bridgeFor({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var view = mainComponent.createObject(null, {})

        compare(view.chosen, null, "no Stoa is chosen at startup")
        compare(view.screenShown, "list", "and no feed is rendered for any Stoa")
        verify(view.stoaAddress === undefined,
               "the view must not carry a Stoa address property of its own")
        verify(view.stoaGenesis === undefined,
               "nor a genesis record property")
        view.destroy()
    }

    Component { id: mainComponent; Main {} }

    function test_the_feed_is_given_the_address_and_the_record_from_the_chosen_row() {
        var addr = "aa".repeat(32)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Held"}],'
                        + '"page":0,"hasMore":false}',
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        var view = mainComponent.createObject(null, {})
        var list = spec.visibleNamed(view, "stoaList")[0]
        var held = {}
        held[addr] = "00ff00ff"
        list.genesisByStoa = held

        list.stoaChosen(addr, "Held", "00ff00ff")

        compare(view.screenShown, "feed")
        compare(view.chosen.stoa, addr)
        compare(view.chosen.genesis, "00ff00ff",
                "the record must travel with the address")
        view.destroy()
    }

    function test_no_record_is_invented_for_a_row_the_view_has_none_for() {
        var addr = "bb".repeat(32)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Not held"}],'
                        + '"page":0,"hasMore":false}',
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"error":"a genesis record is required for this Stoa"}'
        })
        var view = mainComponent.createObject(null, {})
        var list = spec.visibleNamed(view, "stoaList")[0]

        list.stoaChosen(addr, "Not held", list.genesisFor(addr))

        compare(view.chosen.genesis, "",
                "no placeholder or fabricated record may be sent")
        // And whatever the core then refuses is rendered as the failure it is,
        // which is distinguishable from a Stoa holding nothing.
        var feed = spec.visibleNamed(view, "feed")[0]
        compare(feed.readState, "failed")
        view.destroy()
    }

    // ---- behaviour the spec did not decide --------------------------------

    // NO SPEC: the spec requires that what is shared carry both halves and says
    // nothing about the encoding. A one-line JSON object was chosen — see
    // design.md D1 — and this pins the shape so a later change to it is a
    // deliberate one. Anyone who has already copied a reference holds a string
    // in this format, so changing it strands them.
    function test_the_reference_format_is_a_json_object_with_two_hex_fields() {
        var text = StoaReference.shareText("aabb", "ccdd")
        var obj = JSON.parse(text)
        compare(obj.stoa, "aabb")
        compare(obj.genesis, "ccdd")
        compare(Object.keys(obj).length, 2, "no third field, so nothing to ignore")
    }

    // NO SPEC: the spec's three paste outcomes do not cover an EMPTY field. It
    // is refused as not-a-reference rather than silently doing nothing, on the
    // grounds that a button that appears to do nothing is worse than one that
    // says why — but the spec does not require either.
    function test_an_empty_paste_is_refused_rather_than_ignored() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        screen.pasted = ""
        screen.preview()
        verify(screen.pasteFailure.length > 0, "an empty paste says why nothing happened")
        compare(spec.callsTo(bridge, "join_stoa"), 0)
        screen.destroy()
    }

    // NO SPEC: the spec requires the created address be rendered and does not
    // say what happens when a success arrives WITHOUT one. It is treated as a
    // failure here rather than rendering a created Stoa with no address, which
    // would be a Stoa the user cannot name, share or read.
    function test_a_creation_success_carrying_no_address_is_a_failure() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "create_stoa": '{"foundingTitle":"Transport Notes","policy":"open"}'
        })
        screen.createTitle = "Transport Notes"
        screen.create()
        compare(screen.createState, "failed")
        compare(screen.created, null)
        screen.destroy()
    }

    // NO SPEC: the spec says the view's check is limited to "whether the input
    // carries the two halves at all" and does not say what a half that is
    // present but not a string does. A number or an object is refused rather
    // than coerced, so nothing but a string can reach JSON.stringify and the
    // core.
    function test_a_half_that_is_not_a_string_is_refused_rather_than_coerced() {
        compare(StoaReference.parse('{"stoa":12345,"genesis":"00ff"}').ok, false)
        compare(StoaReference.parse('{"stoa":"aa","genesis":{"x":1}}').ok, false)
        compare(StoaReference.parse('["aa","00ff"]').ok, false,
                "an array is not a reference either")
    }

    // ---- rendering, where a property was standing in for it ----------------
    //
    // Every test in this section exists because a property assertion elsewhere
    // in this file passed against a mutation that broke the rendering. The
    // pattern is the one that moved the markup test from `text` to `textFormat`:
    // asking the implementation what it computed and agreeing, where the
    // requirement is about what reaches the screen. Each was proved by mutating
    // a `visible:` binding and watching the whole suite stay green.

    // A share affordance offered for a Stoa the view holds no record for
    // produces a string that fails to verify on somebody else's machine — the
    // third failure this file's header names. `canShare()` answering correctly
    // is not that requirement: the requirement is that no BUTTON is on screen.
    //
    // Proved: `visible: screen.canShare(row.rowStoa)` → `visible: true` on the
    // share button left all 47 prior tests passing.
    function test_no_share_button_is_on_screen_for_a_row_whose_record_is_not_held() {
        var withRecord = "aa".repeat(32)
        var without = "bb".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":['
                        + '{"stoa":"' + withRecord + '","foundingTitle":"Held"},'
                        + '{"stoa":"' + without + '","foundingTitle":"Not held"}'
                        + '],"page":0,"hasMore":false}'
        })
        var held = {}
        held[withRecord] = "00ff00ff"
        screen.genesisByStoa = held
        screen.reload()

        // Two rows, one record. Both halves from one fixture, and the count is
        // hardcoded rather than derived: `<= 1` would pass on zero buttons, and
        // zero is a different defect that this same assertion must catch.
        compare(screen.rows.length, 2, "the fixture must put two rows on screen")
        compare(spec.visibleNamed(screen, "shareButton").length, 1,
                "exactly one share affordance is on screen — the held row's — "
                + "and the unheld row must offer none")
        screen.destroy()
    }

    // The joined outcome has to be REPORTED, which is a thing on screen and not
    // a string in a property. A screen holding `joinState === "joined"` while
    // rendering nothing has told the user nothing.
    //
    // Proved: `visible: screen.joinState === "joined"` → `visible: false` on the
    // joined panel left all 47 prior tests passing, including the two that
    // assert a repeat join is success.
    function test_the_joined_outcome_is_reported_on_screen_and_not_only_in_a_property() {
        var addr = "7f3a91c4" + "ee".repeat(28)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff", foundingTitle: "Nym Research" })

        compare(spec.visibleNamed(screen, "joinedPanel").length, 0,
                "nothing reports a join before the user acts")

        screen.join()

        compare(spec.visibleNamed(screen, "joinedPanel").length, 1,
                "a successful reply must put the outcome on screen, not only in joinState")
        // And the failure panel is not up alongside it. One outcome, not two.
        compare(spec.visibleNamed(screen, "joinFailurePanel").length, 0)
        screen.destroy()
    }

    // "No feed is rendered for any Stoa" before one is chosen. `screenShown` is
    // a derived string; what the spec forbids is a feed on screen.
    //
    // Proved: `visible: root.screenShown === "feed"` → `visible: true` on the
    // FeedScreen left all 47 prior tests passing, with the feed rendering for
    // the empty address at startup.
    function test_no_feed_is_on_screen_before_a_stoa_has_been_chosen() {
        Core.bridge = bridgeFor({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var view = mainComponent.createObject(null, {})

        compare(spec.visibleNamed(view, "feed").length, 0,
                "no feed may be on screen before a Stoa is chosen")
        compare(spec.visibleNamed(view, "joinScreen").length, 0,
                "nor a join preview, which nothing has been previewed for")
        compare(spec.visibleNamed(view, "stoaList").length, 1,
                "the list is the entry point — asserted so that a view rendering "
                + "NOTHING cannot satisfy the two absences above")
        view.destroy()
    }

    // ---- no number appears that nothing computed ---------------------------

    // Every run of digits in what a row renders, in source order.
    function digitRunsIn(text) {
        return text.match(/[0-9]+/g) || []
    }

    // The strengthened count assertion. The dev-writer's version was a blocklist
    // of the mockup's phrasings — `posts received here`, `nothing received yet`,
    // `3 posts` — and said so: it could not catch a count rendered as a bare
    // `31` in a row's margin, which is the shape the mockup actually draws.
    //
    // This is that same requirement stated as a relation instead: **every digit
    // on this screen must be traceable to something the fixture supplied.** The
    // fixture's address carries digits deliberately, so the test is not passing
    // merely because nothing anywhere renders a digit — remove the address from
    // the row and the allowed set shrinks rather than the assertion vanishing.
    //
    // What it still cannot see: a count rendered in a glyph that is not a digit,
    // or spelled out in words. That residue is smaller than a phrase blocklist
    // and is stated rather than left to be assumed.
    function test_no_digit_is_rendered_that_the_reply_did_not_supply() {
        // Digits in the address (4 0 3 9 1 …) and in the title, so the allowed
        // set is non-empty and the test has something to distinguish.
        var addr = "7f3a91c4" + "40".repeat(28)
        var title = "Transport Notes 1972"
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"' + title + '"}],'
                        + '"page":0,"hasMore":false}',
            // A thread listing is answered too, so a screen tempted to render
            // some OTHER call's page length has one available to render.
            "list_threads": '{"items":[{"thread":"t1"},{"thread":"t2"},{"thread":"t3"}],'
                          + '"page":0,"hasMore":true}'
        })
        compare(screen.readState, "ok")
        compare(screen.rows.length, 1, "the fixture must put a row on screen")

        var shown = spec.visibleText(screen)
        var runs = spec.digitRunsIn(shown)
        // Non-empty, or the assertion below is vacuous: a screen rendering no
        // digits at all would satisfy "every digit is traceable" for free, and
        // that is exactly the self-satisfying shape this file is written against.
        verify(runs.length > 0,
               "the fixture's own digits must reach the screen, or this test "
               + "proves nothing: " + shown)

        for (var i = 0; i < runs.length; i++) {
            var run = runs[i]
            var fromAddress = addr.indexOf(run) >= 0
            var fromTitle = title.indexOf(run) >= 0
            verify(fromAddress || fromTitle,
                   "the screen renders the number '" + run + "', which nothing "
                   + "in the reply supplied. No call answers how many posts this "
                   + "peer holds for a Stoa, so any number here was invented or "
                   + "taken from another call's page length. Rendered: " + shown)
        }
        screen.destroy()
    }

    // ---- the two paste outcomes, pinned by meaning rather than by difference

    // The sibling `thread-read` piece shipped three refusal messages asserted to
    // be three DIFFERENT strings; a tester reworded one to actively misinform
    // and both distinguishability tests stayed green, because three misinforming
    // strings are still three distinct strings. So this asserts what each of the
    // two paste outcomes must and must not IMPLY, and the difference between
    // them falls out of that rather than standing in for it.
    //
    // The two mean opposite things about what to do next:
    //   malformed  — the paste went wrong; paste it again.
    //   unverified — somebody handed over a record that is not the one that
    //                address names; do NOT try again.
    function test_a_malformed_paste_and_an_unverified_record_say_different_things_to_do() {
        // 1. Text that is not a reference at all.
        var list = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        list.pasted = "not a reference"
        list.preview()
        var malformed = list.pasteFailure
        verify(malformed.length > 0, "a malformed paste must say something")

        var m = malformed.toLowerCase()
        // It must name the INPUT as the problem — what was pasted — so the
        // reader knows to look at what they pasted.
        verify(m.indexOf("pasted") >= 0 || m.indexOf("paste") >= 0,
               "a malformed paste must name what was pasted as the problem: " + malformed)
        // And it must NOT describe a comparison against the address. A malformed
        // paste has been compared against nothing; saying it failed a check would
        // send a reader who truncated their own paste to blame their sender.
        //
        // **Asserted as a structure rather than as a blocklist.** Three literal
        // phrasings were blocked here before, and a reviewer measured two
        // equivalents that passed — "could not be confirmed against its address"
        // and "failed the check against the address it names" — each carrying
        // exactly the misinformation the blocklist existed to stop. Adding those
        // two would leave a third.
        //
        // A claim of comparison needs both halves: a verb of checking, and the
        // address as the thing checked against. The shipped copy mentions the
        // address freely — it explains what a reference is — so neither half
        // alone can be forbidden. It is the conjunction that is the lie.
        var checkVerb = /\b(hash(es|ed)?|verif(y|ies|ied|ication)|match(es|ed)?|confirm(s|ed)?|check(s|ed)?|validat(e|es|ed|ion))\b/
        var againstAddress = /\b(against|to|with)\b[^.]{0,40}\baddress\b/
        verify(!(checkVerb.test(m) && againstAddress.test(m)),
               "nothing was compared against the address here, so nothing may "
               + "report that a comparison failed — a reader who truncated their "
               + "own paste would be sent to blame their sender: " + malformed)

        // 2. A well-formed pair the core refuses because it does not verify.
        var addr = "b02d5e77" + "88".repeat(28)
        var join = makeJoin({
            "join_stoa": '{"error":"the genesis record does not hash to this address; '
                       + 'whoever sent it did not send the record this address names"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff" })
        join.join()

        var panels = spec.visibleNamed(join, "joinFailureText")
        compare(panels.length, 1, "the refusal must be on screen, not only in a property")
        var refusal = panels[0].text

        // The core's own words, unreworded — they name what was wrong, and that
        // is the difference between a user who knows the record they were sent
        // is wrong and one who thinks the app is broken.
        verify(refusal.indexOf("does not hash to this address") >= 0,
               "the core's refusal must be rendered as it came: " + refusal)
        // It must NOT invite a retry. Pressing again produces the same refusal,
        // and an interface offering one teaches the reader to mistake a
        // permanent answer for a transient fault.
        var retryButtons = spec.visibleNamed(join, "joinRetryButton")
        compare(retryButtons.length, 0, "a verification refusal offers no retry")
        var joinShown = spec.visibleText(join).toLowerCase()
        verify(joinShown.indexOf("try again") < 0 && joinShown.indexOf("retry") < 0,
               "nor may anything on screen suggest one: " + joinShown)

        // And only NOW the difference, which is a consequence of the two
        // meanings above rather than the whole of the assertion.
        verify(refusal.indexOf(malformed) < 0 && malformed.indexOf(refusal) < 0,
               "neither refusal may contain the other")
        join.destroy()
        list.destroy()
    }

    // ---- what the address proves, pinned by what a simplification would lose

    // The spec deliberately does not pin this copy's wording, because the thing
    // it has to convey is a distinction: the check is a hash comparison between
    // TWO INPUTS THE USER SUPPLIED, so a reader who pasted a hostile address and
    // saw a verified record has verified the attacker's record against the
    // attacker's address, perfectly successfully.
    //
    // The dev-writer's version pins three substrings, which fails on a reword
    // and passes on a misinformation. This asserts the two halves the copy
    // cannot lose — the scope of the proof, and the named remainder — and
    // requires that no unqualified claim of verification stands anywhere on the
    // screen. Someone simplifying this copy to "Verified." satisfies none of
    // the three.
    function test_the_address_note_cannot_be_simplified_into_an_unqualified_verified() {
        var screen = makeJoin({}, { stoaAddress: "aa".repeat(32), stoaGenesis: "00ff",
                                    foundingTitle: "Nym Research" })
        var notes = spec.visibleNamed(screen, "addressNote")
        compare(notes.length, 1, "the explanation must be on screen")
        var note = notes[0].text
        var n = note.toLowerCase()

        // 1. The proof is SCOPED to the record this address names. Not to the
        //    Stoa, its creator, or its reachability.
        verify(n.indexOf("this address names") >= 0 || n.indexOf("the record shown") >= 0,
               "the note must say what is proved: that the record shown is the "
               + "one this address names. Got: " + note)

        // 2. The remainder is NAMED, not merely unmentioned. An absence
        //    assertion here would pass on copy that said nothing at all, which
        //    is the failure: a reader told nothing assumes everything checked.
        verify(n.indexOf("unverified") >= 0 || n.indexOf("not verified") >= 0
               || n.indexOf("not checked") >= 0,
               "the note must NAME the unverified remainder rather than leave it "
               + "unmentioned. Got: " + note)

        // 3. And the scope is bounded by an explicit negation. Copy that states
        //    the positive and stops has told the reader they finished checking.
        verify(n.indexOf("nothing more") >= 0 || n.indexOf("not checked against") >= 0
               || n.indexOf("no registry") >= 0 || n.indexOf("any registry") >= 0,
               "the note must bound what the check consulted — no registry, no "
               + "peer, no third party. Got: " + note)

        // 3b. **Provenance is denied, in so many words.** This is the check the
        //     four above could not make, and a reviewer proved it: the note
        //     "The record shown is the one this address names, which confirms
        //     this is the Stoa you were sent. Nothing more is needed. No registry
        //     was consulted because none is needed; only the founding title is
        //     unverified." satisfies every one of them — it carries `this address
        //     names`, `unverified`, `no registry`, and dodges the three literal
        //     phrases below — while telling the reader the address is the one
        //     they were meant to receive.
        //
        //     That is THE claim spec.md's "What the address proves is stated
        //     exactly, and nothing broader" forbids, because it is the whole of
        //     the residual risk: a reader who pastes a hostile address and is
        //     shown a verified record has verified the attacker's record against
        //     the attacker's address, perfectly successfully. Assembling more
        //     required substrings cannot catch it — the hostile note contains
        //     every one. Only the negation can, so the negation is required.
        verify(/\bnothing\b[^.]{0,60}\b(says|establishes|proves|shows|tells)\b/.test(n)
               || /\bdoes not\b[^.]{0,60}\b(establish|prove|show|confirm|mean)\b/.test(n)
               || /\bcannot\b[^.]{0,60}\b(establish|prove|show|confirm|tell)\b/.test(n),
               "the note must explicitly DENY provenance — that nothing here "
               + "establishes this is the address you were meant to receive — "
               + "rather than merely omitting the claim. Got: " + note)
        // And the affirmative form of that claim is refused outright, however
        // the sentence around it is built.
        verify(!/\b(confirms|proves|establishes|means)\b[^.]{0,60}\b(you were sent|meant to receive|the right|the one you wanted)\b/.test(n),
               "nothing may claim this is the address the reader was sent or "
               + "meant to receive. Got: " + note)

        // 4. Nothing anywhere on the screen makes an unqualified claim of
        //    verification. This is the assertion a "Verified." simplification
        //    fails: the founding title is on screen beside it, freely chosen and
        //    matched against nothing, so a bare "verified" attaches to it.
        //
        //    This scan covers every visible Text on the screen, which today
        //    includes the apparatus column. **Nothing in the apparatus is what
        //    satisfies these assertions** — checks 1 to 3 read `addressNote` by
        //    name, and no apparatus note contains "verified" — so removing the
        //    apparatus column weakens none of them. Said explicitly because a
        //    whole-screen scan otherwise leaves that unanswerable by reading.
        var shown = spec.visibleText(screen).toLowerCase()
        verify(shown.indexOf("verified") < 0 || shown.indexOf("unverified") >= 0
               || shown.indexOf("not verified") >= 0,
               "the word 'verified' may not stand on this screen without the "
               + "remainder being named alongside it: " + shown)
        verify(shown.indexOf("this stoa is verified") < 0
               && shown.indexOf("verified stoa") < 0
               && shown.indexOf("verified record") < 0,
               "nothing may present the Stoa or its record as verified outright: " + shown)
        screen.destroy()
    }

    // ---- scanned over the screen the requirement names ---------------------
    //
    // R13 forbids a moderator or per-Stoa-identity claim on THESE SCREENS, and
    // its scenario (spec.md:611) names two: the list rendering a Stoa, and the
    // creation outcome rendering a newly created one. The only test on the
    // subject scanned the JOIN screen — a third screen the scenario does not
    // mention — so the requirement was pinned over the wrong corpus entirely.
    //
    // Measured by a reviewer: captioning the creation outcome `CREATED — THIS IS
    // ITS ADDRESS. You moderate this Stoa, and joining it generates you an
    // identity for it alone.` left **all 100 tests passing**, with the sentence
    // confirmed rendered. Both claims are forbidden, and the second is the one
    // the spec calls the only failure here that could actually harm someone.
    //
    // This is the corpus defect the file header documents, recurring in the
    // requirement the header calls the most harmful to get wrong. Hence the
    // corpus assertion below, before any absence: it is the lesson applied to
    // its own fix.
    function test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_identity() {
        var addr = "b02d5e77" + "a1".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Transport Notes"}],'
                        + '"page":0,"hasMore":false}',
            "create_stoa": '{"stoa":"' + addr + '","foundingTitle":"Transport Notes","policy":"open"}'
        })
        screen.createTitle = "Transport Notes"
        screen.create()
        compare(screen.createState, "created", "the creation outcome must be on screen")

        var body = spec.bodyText(screen)

        // **The corpus, asserted before the absences.** Both screens the
        // scenario names must actually be rendering: the list's row, and the
        // creation outcome. Without this the test would pass on a blank screen,
        // which is exactly how its sibling proved nothing for so long.
        verify(body.indexOf("CREATED") >= 0,
               "the creation outcome must be in the scanned corpus, or the "
               + "absences below prove nothing: " + body)
        verify(body.indexOf("Transport Notes") >= 0,
               "and the list's row with it: " + body)

        var shown = body.toLowerCase()

        // The same two predicates the join screen uses, over the screens R13's
        // scenario actually names. Deliberately the same words: one requirement,
        // one vocabulary, so a reviewer comparing the two reads them as the pair
        // they are.
        verify(!/\bidentity\b/.test(shown),
               "neither the list nor the creation outcome may raise identity — "
               + "one key signs in every Stoa in this release, so a per-Stoa "
               + "identity promise offers an unlinkability property the "
               + "software does not have: " + body)
        verify(shown.indexOf("you moderate") < 0,
               "nor may either claim the user moderates a Stoa — the creator key "
               + "in the record is never re-checked against this peer's current "
               + "signing key, so it is a question these screens cannot answer: "
               + body)
        // A created Stoa is the tempting case for the moderation claim, since
        // this peer did create it. Refused in the other voice too.
        verify(!/\byou (are|become|are now)\b[^.]{0,40}\bmoderator\b/.test(shown),
               "including of a Stoa this peer just created: " + body)
        screen.destroy()
    }

    // R11's central prohibition — "it offers no field for a creator key or an
    // identity to create under" — was pinned by nothing. The only test on the
    // create affordance asserts the button exists and is enabled, which is the
    // scenario's FIRST clause and never its second.
    //
    // Measured by a reviewer: a visible `TextInput` labelled `CREATOR KEY` beside
    // the title field left all 100 tests passing, confirmed rendered.
    //
    // The stake is why this is not a nicety: the creator key is fixed inside the
    // address preimage forever, so a field for one mints a Stoa nobody can
    // moderate at an address that cannot be un-minted.
    function test_the_create_affordance_offers_exactly_one_field_and_it_is_not_a_key() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })

        // **Counted, not searched by name.** A test looking for an element named
        // `creatorKeyField` pins one spelling of the defect and nothing else;
        // the next one would be called `identityPicker`. What the requirement
        // bounds is how many things the user can type into at all.
        //
        // The reviewer's warning applies here and shaped this: the shipped title
        // field is a bare `TextInput` with no `placeholderText`, so a probe keyed
        // on that property finds nothing even on a clean tree. These three
        // properties are what an editable text input exposes and a `Text` does
        // not — verified by walking the shipped screen.
        var inputs = spec.editableInputs(screen)

        // Two: the create title, and the paste field. Hardcoded rather than
        // `<= 2`, because zero is a different defect — a create affordance with
        // no title field at all — and an inequality would pass on it.
        compare(inputs.length, 2,
                "exactly two things on this screen accept typing: the Stoa title "
                + "and the paste field. A third is a field for something the "
                + "create call does not take — and the only candidates are a "
                + "creator key or an identity, neither of which is a parameter "
                + "and neither of which can be: " + spec.describeInputs(inputs))

        // And nothing captions a field as a key or an identity, which catches a
        // key field built from something other than a TextInput.
        var shown = spec.visibleText(screen).toLowerCase()
        verify(!/\b(creator key|signing key|private key|secret key)\b/.test(shown),
               "no field may be captioned as a key: " + shown)
        verify(!/\b(identity|identities)\b[^.]{0,30}\b(to create|create under|choose|select|pick)\b/.test(shown),
               "nor may an identity be offered to create under: " + shown)

        // The affordance is still THERE — asserted alongside, so a screen that
        // rendered no create row at all could not satisfy the absences above.
        compare(spec.visibleNamed(screen, "createStoaButton").length, 1,
                "the create affordance itself must be present")
        screen.destroy()
    }

    // R1: "A second abbreviation MUST NOT be written." The existing row test
    // says in its own comment that it asserts "the ADDRESS is there in some
    // form, not that a second elision was written" — so the prose requirement
    // was knowingly unpinned, and a reviewer confirmed it: replacing the row's
    // `AddressLabel` with a hand-rolled `head8 + "…" + tail6` left all 100 tests
    // passing. It passes because both the right and the wrong implementation
    // render the head `7f3a91c4`, which is the only thing asserted — the repo's
    // defect family exactly, a question both implementations answer alike.
    //
    // The requirement says why it matters: "an elision that keeps only a head
    // and a tail is the shape vanity-address generators are built to defeat".
    function test_a_row_abbreviates_through_the_one_component_and_keeps_a_middle_group() {
        // Every 8-character window of this address is distinct, so "the middle
        // group came from the middle" is a real check rather than one satisfied
        // by a repeated byte matching an earlier offset by accident.
        var addr = "7f3a91c4" + "0123456789abcdef" + "fedcba9876543210"
                 + "13579bdf02468ace" + "cafebabe"
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Transport Notes"}],'
                        + '"page":0,"hasMore":false}'
        })
        compare(screen.rows.length, 1, "the fixture must put a row on screen")

        // The row's address element, found by carrying the address rather than
        // by a name the mutation could keep.
        var labels = spec.addressElementsFor(screen, addr)
        compare(labels.length, 1,
                "exactly one element renders this row's address: "
                + spec.visibleText(screen))
        var label = labels[0]

        // **Two assertions, and they fail for different reasons.**
        //
        // (a) The rendered form keeps a MIDDLE group. This is the requirement
        //     itself rather than a proxy for it: head-8 middle-8 tail-6 renders
        //     three groups, a head-and-tail elision renders two. A correct
        //     reimplementation would satisfy this and a weak one cannot,
        //     whatever component it lives in.
        var parts = label.text.split("…")
        compare(parts.length, 3,
                "the abbreviation must render THREE groups separated by two "
                + "ellipses — head, middle, tail. A head-and-tail form is the "
                + "shape vanity generators are built to defeat. Got: " + label.text)
        verify(parts[1].length > 0, "the middle group must not be empty")
        // The middle group is drawn from the middle of the address, so it is
        // neither the head nor the tail — asserted as a relation, because
        // hardcoding the offset would re-implement AddressLabel's arithmetic
        // here and agree with it by construction.
        verify(addr.indexOf(parts[1]) > parts[0].length,
               "the middle group must come from the middle of the address, "
               + "not be a second copy of the head: " + label.text)

        // (b) And it is rendered through the component that OWNS that form.
        //     `full` is a property AddressLabel declares and a bare Text does
        //     not, so this is the "no second implementation" clause: a screen
        //     that grew its own elision fails here even if it happened to keep
        //     three groups.
        verify(label.full !== undefined,
               "the row's address must render through AddressLabel, which owns "
               + "the 8-8-6 form — a second implementation is how one screen "
               + "quietly acquires the weaker one")
        compare(label.full, false, "and a row abbreviates rather than showing it whole")
        screen.destroy()
    }

    // ---- every call goes through the one wrapper --------------------------

    function test_each_core_method_is_named_once_and_reached_through_the_wrapper() {
        // The wrapper is what normalises every reply into one of two shapes, and
        // it is the only thing that makes these screens testable at all — a
        // screen calling the host directly is a screen whose failure paths
        // cannot be exercised anywhere. If these calls did not go through
        // Core.bridge, every fake above would have been ignored and every test
        // in this file would be asserting against a real host that is not there.
        Core.bridge = bridgeFor({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        var screen = listComponent.createObject(null, {})
        compare(spec.callsTo(bridge, "list_stoas"), 1)
        screen.destroy()

        Core.bridge = bridgeFor({ "create_stoa": '{"stoa":"aa","foundingTitle":"t"}' })
        bridge = Core.bridge
        var out = Core.createStoa("t")
        compare(spec.callsTo(bridge, "create_stoa"), 1)
        compare(out.ok, true)

        Core.bridge = bridgeFor({ "join_stoa": '{"stoa":"aa","foundingTitle":"t"}' })
        bridge = Core.bridge
        Core.joinStoa("aa", "00ff")
        compare(spec.callsTo(bridge, "join_stoa"), 1)
        verify(String(spec.lastArgsTo(bridge, "join_stoa")).indexOf('"genesis":"00ff"') >= 0,
               "both halves must reach the core")
    }
}
