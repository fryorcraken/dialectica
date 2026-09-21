import QtQuick
import QtTest
import "../src/qml"

// The thread screen's place in the navigator.
//
// `Main.qml`'s invariant is that the navigator's whole state is which property
// is non-null, with no StackView — so a push/pop lifecycle cannot disagree with
// it. The thread screen has to hold to that.
//
// **THE MECHANISM UNDER TEST CHANGED WHEN `piece/ui-navigation` MERGED, and
// three assertions here were rewritten rather than deleted.** This piece and
// that one each built a thread route independently. This file was written
// against THIS piece's `Main.qml`, where `openThread()` left `chosen` set so the
// feed stayed live "underneath" the thread. The merged navigator does the
// opposite: every setter clears what the others own, so `openThread()` clears
// `chosen` and `closeThread()` restores it from what `reading` carried.
//
// `design.md` D9 settles which wins — "the navigation piece's spec governs the
// transition, and the thread piece owns the destination" — so the merged
// mechanism is the contracted one and the three assertions below were asserting
// a mechanism that no longer exists. Each now asserts the OUTCOME the spec
// actually requires (the user reaches the feed they came from, without the view
// restarting), which both mechanisms had to satisfy and only one of which is
// still built. `tst_navigation.qml` is the file that covers the transition in
// full; this one covers the screen's place in it.
//
// Recorded at this length because a deleted assertion and a superseded one look
// identical in a diff six months from now.
TestCase {
    id: spec
    name: "ThreadNavigation"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
        // Every screen in Main probes core on construction. A bridge that
        // answers nothing leaves each in its own failed state, which is fine —
        // what is under test is the navigator, not any screen's read.
        Core.bridge = {
            callModule: function (module, method, args) {
                if (method === "get_capabilities")
                    return '{"canPost":false,"reason":"no keystore"}'
                if (method === "list_stoas")
                    return '{"items":[],"page":0,"hasMore":false}'
                return '{"error":"not wired in this test"}'
            }
        }
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    Component {
        id: mainComponent
        Main {}
    }

    function test_the_view_starts_on_the_list_with_no_thread() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        compare(main.screenShown, "list")
        compare(main.reading, null, "no thread is open before one is chosen")
        main.destroy()
    }

    function test_opening_a_thread_shows_the_thread_screen() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        main.open("ab".repeat(32), "Agora", "00ff")
        compare(main.screenShown, "feed")

        main.openThread("root1")
        compare(main.screenShown, "thread",
                "the thread is tested first, because it sits above the feed")
        main.destroy()
    }

    // SUPERSEDED MECHANISM, REWRITTEN. This asserted `chosen !== null` while a
    // thread was open — this piece's "the feed stays live underneath" shape. The
    // merged navigator clears `chosen` and carries the feed's context inside
    // `reading` instead, so that assertion now fails against a correct tree.
    //
    // What the spec actually requires is that the feed the thread was opened
    // from is RECOVERABLE, not that it is held in one particular property. That
    // is what is asserted now, and it is the stronger claim: it holds under
    // either mechanism, where the old one pinned the implementation.
    function test_the_feed_it_was_opened_from_is_recoverable_while_reading() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        main.open("ab".repeat(32), "Agora", "00ff")
        main.openThread("root1")

        verify(main.reading !== null, "a thread is open")
        compare(main.reading.stoa, "ab".repeat(32),
                "and it carries the Stoa of the feed it was opened from, which "
                + "is what makes the return a return rather than a reset")
        compare(main.reading.genesis, "00ff",
                "with that Stoa's founding record, travelling as held")
        main.destroy()
    }

    function test_leaving_the_thread_returns_to_the_feed_it_was_opened_from() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        main.open("ab".repeat(32), "Agora", "00ff")
        main.openThread("root1")
        main.closeThread()

        compare(main.screenShown, "feed", "the feed is rendered again")
        compare(main.chosen.stoa, "ab".repeat(32),
                "and it is the same feed, not a rebuilt one")
        main.destroy()
    }

    // SUPERSEDED MECHANISM, REWRITTEN. This called `closeFeed()` while a thread
    // was open and asserted it cleared `reading` too — meaningful only under the
    // old shape, where both properties were set at once. Under the merged
    // navigator that state cannot be constructed in the first place, so the
    // assertion was testing a guard against a state that no longer exists.
    //
    // The invariant it was defending is real and is asserted directly instead:
    // no two of the navigator's state properties are ever set together, which is
    // what makes `screenShown`'s ternary a rendering of the state rather than a
    // resolution of a conflict.
    function test_no_two_navigator_states_are_set_at_once() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        function setCount() {
            return (main.chosen !== null ? 1 : 0)
                 + (main.previewing !== null ? 1 : 0)
                 + (main.reading !== null ? 1 : 0)
                 + (main.onboarding !== null ? 1 : 0)
        }

        main.open("ab".repeat(32), "Agora", "00ff")
        compare(setCount(), 1, "the feed alone")

        main.openThread("root1")
        compare(setCount(), 1,
                "opening a thread clears the feed rather than adding to it — "
                + "the state with both set has no rendering")

        main.closeThread()
        compare(setCount(), 1, "and leaving it restores exactly one state")
        compare(main.screenShown, "feed")

        main.closeFeed()
        compare(setCount(), 0, "and leaving the feed leaves the list")
        compare(main.screenShown, "list")
        main.destroy()
    }

    function test_no_thread_opens_without_a_feed_under_it() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        main.openThread("root1")

        compare(main.reading, null,
                "a thread needs the feed that names its Stoa; there is no second source")
        compare(main.screenShown, "list")
        main.destroy()
    }

    function test_an_empty_thread_id_opens_nothing() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        main.open("ab".repeat(32), "Agora", "00ff")
        main.openThread("")

        compare(main.reading, null,
                "a row naming no op reaches no thread screen, so no read is made "
                + "for a thread the user never asked for")
        compare(main.screenShown, "feed")
        main.destroy()
    }

    // What travels across the transition is the feed's own Stoa, record and
    // title — nothing is supplied a second time, so there is no opportunity to
    // substitute a placeholder record.
    function test_the_thread_is_given_the_feeds_stoa_and_record() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        main.open("ab".repeat(32), "Agora", "00ff")
        main.openThread("root1")

        var thread = findChild(main, "thread")
        verify(thread !== null)
        compare(thread.stoaAddress, "ab".repeat(32))
        compare(thread.stoaGenesis, "00ff",
                "the founding record travels as held, never invented")
        compare(thread.threadId, "root1")
        main.destroy()
    }

    // A Stoa the view holds no record for opens the thread WITHOUT one, and
    // whatever core then refuses is rendered as the refusal it is. A fabricated
    // record would fail verification and surface as a failure the user cannot
    // act on.
    function test_no_record_is_invented_for_a_stoa_holding_none() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        main.open("ab".repeat(32), "Agora", "")
        main.openThread("root1")

        var thread = findChild(main, "thread")
        compare(thread.stoaGenesis, "",
                "no placeholder record is substituted for an absent one")
        main.destroy()
    }

    function test_only_one_screen_is_visible_at_a_time() {
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })

        main.open("ab".repeat(32), "Agora", "00ff")
        main.openThread("root1")

        var names = ["stoaList", "joinScreen", "feed", "thread"]
        var visible = 0
        for (var i = 0; i < names.length; i++) {
            var screen = findChild(main, names[i])
            if (screen !== null && screen.visible)
                visible += 1
        }
        compare(visible, 1,
                "the navigator's whole state is which property is non-null, so "
                + "exactly one screen renders")
        main.destroy()
    }
}
