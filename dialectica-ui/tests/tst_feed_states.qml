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

    function test_a_missing_stoa_address_is_not_a_silent_empty() {
        Core.bridge = bridgeFor({})
        var screen = feedComponent.createObject(null, { stoaAddress: "" })

        verify(screen.readState !== "ok",
               "a view given no Stoa must say so rather than look empty")
        screen.destroy()
    }

    // ---- "we never asked" is its own state, not a storage failure --------
    //
    // THE COPY BUG, and it is worse than a layout defect because it invents a
    // fact about the user's data.
    //
    // `reload()` short-circuits when `stoaAddress` is empty — it returns BEFORE
    // touching the bridge. Nothing was read; there is no store involved. But the
    // old code set `readState = "failed"`, which renders the storage-failure
    // panel and with it two fabricated sentences: "The store could not be read"
    // and "This is not an empty Stoa. Posts you already hold are on disk and
    // unreadable right now." The only true line, "No Stoa address was given to
    // this view", was the smallest text in the panel.
    //
    // UI-BRIEF obligation 5 exists to keep "empty" and "unreadable"
    // distinguishable. A third case — *we never asked* — was being rendered as
    // the second, which defeats the obligation from a direction it did not
    // anticipate.
    //
    // The predecessor of this test pinned only `readState === "failed"` and so
    // let the wrong copy through entirely. That is this project's recorded defect
    // family: a fixture where two explanations give the same answer.

    function test_no_address_is_its_own_state_and_not_a_storage_failure() {
        Core.bridge = bridgeFor({})
        var screen = feedComponent.createObject(null, { stoaAddress: "" })

        compare(screen.readState, "unasked",
                "no address supplied means the store was never consulted, so "
                + "this must NOT be the state that renders 'the store could not "
                + "be read' — nothing was read")
        screen.destroy()
    }

    function test_the_no_address_state_never_claims_anything_about_the_store() {
        // The property stated over the strings themselves rather than over the
        // state name, because the state name is not what the user reads. A
        // future change that renamed the state but kept the copy would pass the
        // test above and still lie on screen.
        Core.bridge = bridgeFor({})
        var screen = feedComponent.createObject(null, { stoaAddress: "" })

        // Asserted present BEFORE asserting absent. Without this the whole test
        // passes vacuously while the properties are undefined — "undefined"
        // contains none of the forbidden substrings, so a screen that renders no
        // copy at all would look compliant. Watched it do exactly that.
        verify(screen.statusTitle !== undefined && screen.statusTitle.length > 0,
               "the no-address state must SAY something; statusTitle was: "
               + screen.statusTitle)
        verify(screen.statusBody !== undefined && screen.statusBody.length > 0,
               "the no-address state must explain itself; statusBody was: "
               + screen.statusBody)

        var said = screen.statusTitle + " " + screen.statusBody
        verify(said.indexOf("store could not be read") < 0,
               "the no-address state must not claim a failed read; it said: "
               + said)
        verify(said.indexOf("on disk") < 0,
               "the no-address state must not claim anything about what is on "
               + "disk — nothing was examined. It said: " + said)
        verify(said.indexOf("not an empty Stoa") < 0,
               "the no-address state must not assert the Stoa is non-empty; no "
               + "Stoa was named. It said: " + said)
        screen.destroy()
    }

    function test_a_real_store_failure_still_says_the_store_failed() {
        // The counter-pressure, so the fix above cannot be satisfied by removing
        // the storage-failure language altogether. When a read genuinely fails,
        // the panel must still say so — that is obligation 5's other half.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"error":"the database at /x/ops.sqlite is locked"}'
        })

        compare(screen.readState, "failed")
        var said = screen.statusTitle + " " + screen.statusBody
        verify(said.indexOf("store could not be read") >= 0,
               "a real read failure must still name itself as one; it said: "
               + said)
        screen.destroy()
    }

    // ---- the ordering label ---------------------------------------------

    function test_no_ordering_claims_to_be_the_same_for_everyone() {
        // "same order for everyone" was rendered as the ordering's label. It is
        // false as an interface promise: UI-BRIEF is explicit that the
        // convergence is "a property of *this fallback*, not of Dialectica", and
        // that such a label "must not become the interface's general promise" —
        // once vote-weighting exists, vouching is per-reader and never
        // published, so two readers legitimately compute different orders over
        // identical ops.
        //
        // With exactly one ordering there is also nothing to choose between, so
        // the label was furniture that asserted a falsehood.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })

        for (var i = 0; i < screen.orderings.length; i++) {
            var label = screen.orderings[i].label
            var text = label === undefined ? "" : String(label)
            verify(text.indexOf("same order for everyone") < 0,
                   "ordering " + i + " is labelled '" + text + "' — that states "
                   + "as a general property something true only of the degraded "
                   + "op-id fallback")
            verify(text.indexOf("everyone") < 0,
                   "ordering " + i + " is labelled '" + text + "' — no ordering "
                   + "may promise anything about what other readers see")
        }
        screen.destroy()
    }

    function test_the_ordering_row_is_still_model_driven() {
        // The counter-pressure to the test above: UI-BRIEF requires that "the
        // labels must be able to change when the real ordering arrives, without
        // the layout changing around them", so the row stays a model rather than
        // being deleted. Removing the false label must not foreclose a second
        // ordering appearing later.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })

        verify(Array.isArray(screen.orderings),
               "the orderings must remain a model an ordering can be added to")
        compare(screen.orderings.length, 1,
                "core implements exactly one ordering today")
        verify(screen.orderings[0].key !== undefined,
                "an ordering must still be identifiable by key, so a future "
                + "selection control has something to select")
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
