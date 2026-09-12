import QtQuick
import QtTest
import "../src/qml"

// The three states of a feed read, and the one distinction the screen exists
// to make: **an empty store and an unreadable store must never look alike.**
//
// UI-BRIEF obligation 5 states it, core keeps the two reply shapes disjoint
// (with a wire test pinning that), and `reload()` has a guard so the screen does
// not depend on a promise made one module away. Nothing on the QML side checked
// any of it until this file.
//
// These tests drive the real FeedScreen through a fake bridge, so what is under
// test is the screen's own state machine rather than a re-implementation of it.
TestCase {
    id: spec
    name: "FeedStates"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    // A bridge that answers each method from a map, so one fake can serve the
    // capability probe and the feed read in a single reload().
    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    // `stoaAddress` must be non-empty or reload() short-circuits before it ever
    // calls the bridge — which would make every test below pass for the wrong
    // reason.
    function makeScreen(replies) {
        Core.bridge = bridgeFor(replies)
        return feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora"
        })
    }

    Component {
        id: feedComponent
        FeedScreen {}
    }

    // ---- the empty state ------------------------------------------------

    function test_an_empty_store_is_the_ok_state_with_no_rows() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })

        compare(screen.readState, "ok",
                "a store that answered with nothing is a SUCCESS, not a failure")
        compare(screen.rows.length, 0)
        compare(screen.failure, "", "an empty read has nothing to report")
        screen.destroy()
    }

    // ---- the failed state -----------------------------------------------

    function test_a_store_failure_is_the_failed_state_and_never_an_empty_feed() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "list_threads": '{"error":"the database at /x/ops.sqlite is locked"}'
        })

        compare(screen.readState, "failed")
        compare(screen.rows.length, 0, "a failure must not leave rows behind")
        verify(screen.failure.indexOf("locked") >= 0,
               "core's reason must reach the screen so it can be shown, got: "
               + screen.failure)
        screen.destroy()
    }

    function test_the_two_states_are_distinguishable() {
        // The property stated directly rather than inferred from the two tests
        // above: these are the same request against two stores, and the screen
        // must not end up in the same state.
        var empty = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        var broken = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"error":"database is locked"}'
        })

        verify(empty.readState !== broken.readState,
               "empty and unreadable must never reach the same state")
        compare(empty.readState, "ok")
        compare(broken.readState, "failed")
        empty.destroy()
        broken.destroy()
    }

    // ---- the guard added for the review -------------------------------

    function test_a_success_without_an_items_array_becomes_a_named_failure() {
        // THE guard. Core keeps the shapes disjoint, but this screen's entire
        // purpose is that the two do not look alike, so it does not rest on a
        // guarantee made in another module. A reply that is neither shape must
        // be a failure — assigning `undefined` to `rows` would render as an
        // empty feed, which is the exact confusion being prevented.
        var noItems = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"page":0,"hasMore":false}'
        })
        compare(noItems.readState, "failed",
                "a success with no items array must not render as an empty feed")
        verify(noItems.failure.length > 0, "the failure must name itself")
        noItems.destroy()

        // And `items` present but not an array — a shape that would pass an
        // `=== undefined` check alone and then have `.length` read off it.
        var notArray = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":"lots","page":0,"hasMore":false}'
        })
        compare(notArray.readState, "failed",
                "items must be an ARRAY, not merely present")
        notArray.destroy()
    }

    function test_a_missing_stoa_address_is_a_failure_rather_than_a_silent_empty() {
        Core.bridge = bridgeFor({})
        var screen = feedComponent.createObject(null, { stoaAddress: "" })

        compare(screen.readState, "failed",
                "a view given no Stoa must say so rather than look empty")
        verify(screen.failure.length > 0)
        screen.destroy()
    }

    // ---- rows, and what a row carries ----------------------------------

    function test_rows_are_taken_from_the_reply_in_order() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":['
                + '{"thread":"t1","currentVersion":"t1","author":"a1",'
                + '"body":{"text":"first","removed":0,"marked":0},'
                + '"attachments":[],"isRevised":false,"isHidden":false},'
                + '{"thread":"t2","currentVersion":"v2","author":"a2",'
                + '"body":{"text":"second","removed":0,"marked":0},'
                + '"attachments":[],"isRevised":true,"isHidden":false}'
                + '],"page":0,"hasMore":true}'
        })

        compare(screen.readState, "ok")
        compare(screen.rows.length, 2)
        compare(screen.rows[0].thread, "t1", "order must be the reply's order")
        compare(screen.rows[1].thread, "t2")
        compare(screen.rows[1].isRevised, true)
        compare(screen.hasMore, true, "hasMore must come from the reply")
        screen.destroy()
    }

    function test_has_more_defaults_to_false_rather_than_undefined() {
        // `visible:` bindings read this. An undefined would be falsy by
        // accident rather than by decision, and would flip if the field's
        // absence ever meant something else.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0}'
        })
        compare(screen.hasMore, false)
        screen.destroy()
    }

    // ---- the posting gate ----------------------------------------------

    function test_the_gate_is_closed_when_the_probe_says_so() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"No keystore found."}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        compare(screen.capability.canPost, false)
        compare(screen.capability.reason, "No keystore found.",
                "the reason names a fix and must not be reworded by the view")
        screen.destroy()
    }

    function test_the_gate_is_open_only_when_the_probe_says_it_is() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"deadbeef"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        compare(screen.capability.canPost, true)
        screen.destroy()
    }

    function test_a_failed_probe_closes_the_gate_rather_than_opening_it() {
        // Fail CLOSED. A probe that could not be reached must not leave a
        // compose affordance on screen — the whole point of gating is that a
        // box the user types into can actually submit.
        var screen = makeScreen({
            "get_capabilities": '{"error":"the keystore could not be read"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        compare(screen.capability.canPost, false,
                "an unreachable probe must not open the gate")
        verify(screen.capability.reason.length > 0)
        screen.destroy()
    }
}
