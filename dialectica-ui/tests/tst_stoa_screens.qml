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
        verify(shown.indexOf("members") < 0 && shown.indexOf("peers reachable") < 0,
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
        var screen = makeJoin({}, { stoaAddress: "aa".repeat(32), stoaGenesis: "00ff" })
        var shown = spec.visibleText(screen).toLowerCase()

        // The bundle's apparatus note ends "and generates you an identity for it
        // alone". One key signs in every Stoa in this release, so that sentence
        // promises an unlinkability property the software does not have — the
        // one false claim on these screens that could actually harm somebody.
        verify(shown.indexOf("identity for it alone") < 0,
               "no per-Stoa identity may be promised")
        verify(shown.indexOf("an identity for that stoa") < 0)
        verify(shown.indexOf("you moderate") < 0, "no moderator status may be asserted")
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
        var shown = spec.visibleText(screen).toLowerCase()
        verify(shown.indexOf("already") < 0, "no warning about a repeat")
        verify(shown.indexOf("collision") < 0 && shown.indexOf("duplicate") < 0,
               "and no collision to resolve: " + shown)
        compare(spec.visibleNamed(screen, "joinFailurePanel").length, 0)
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
