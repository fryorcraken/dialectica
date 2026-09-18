import QtQuick
import QtTest
import "../src/qml"

// The thread screen's states, and the two distinctions it exists to keep.
//
//   - **A refused read and a thread with no replies are different screens.** A
//     reader shown an empty thread when the thread was never received concludes
//     a post vanished.
//   - **A withheld body and an empty body are different facts.** The
//     distinction survives the whole core path — `body` is OMITTED where
//     withheld and `{"text":""}` where an author cleared it — and is destroyed
//     at the last step if the view renders both blank.
TestCase {
    id: spec
    name: "ThreadStates"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    function makeScreen(replies) {
        Core.bridge = bridgeFor(replies)
        return threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora",
            threadId: "root1"
        })
    }

    function threadOf(items) {
        return JSON.stringify({ items: items, page: 0, hasMore: false })
    }

    function rootItem() {
        return {
            thread: "root1", id: "root1", currentVersion: "root1",
            author: "aa".repeat(32), isRevised: false,
            moderation: { state: "unmoderated" },
            position: "0",
            body: { text: "the root post", removed: 0, marked: 0 }
        }
    }

    Component {
        id: threadComponent
        DThreadScreen {}
    }

    // ---- refused vs. no replies ------------------------------------------

    function test_a_refused_read_is_the_failed_state_and_renders_no_items() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": '{"error":"this peer holds no op under abc, so there is no thread to read; it may not have arrived yet"}'
        })

        compare(screen.readState, "failed")
        compare(screen.items.length, 0, "a refusal renders no thread items")
        screen.destroy()
    }

    function test_a_refusal_shows_cores_message_as_supplied() {
        var message = "the post abc is a reply rather than a thread's root; read the thread its parent chain reaches instead"
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": JSON.stringify({ error: message })
        })

        compare(screen.failure, message,
                "core's message is rendered as supplied, never reworded")
        screen.destroy()
    }

    // Core contracts three distinguishable refusals that reach a view only as
    // differing prose inside one error shape. A view branching on that prose
    // would turn every improvement to the wording into a silent breaking change.
    function test_three_refusals_reach_the_same_state_no_branch_on_text() {
        var messages = [
            "this peer holds no op under abc, so there is no thread to read; it may not have arrived yet",
            "the op abc is not a post, so it opens no thread",
            "the post abc is a reply rather than a thread's root; read the thread its parent chain reaches instead"
        ]

        for (var i = 0; i < messages.length; i++) {
            var screen = makeScreen({
                "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
                "read_thread": JSON.stringify({ error: messages[i] })
            })

            compare(screen.readState, "failed",
                    "every refusal reaches the same state")
            compare(screen.failure, messages[i],
                    "and each message is rendered as supplied")
            compare(screen.items.length, 0)
            screen.destroy()
        }
    }

    function test_a_thread_holding_only_its_root_is_a_success() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem()])
        })

        compare(screen.readState, "ok",
                "a thread with no replies is a SUCCESS, not a failure")
        compare(screen.items.length, 1, "the root is rendered")
        compare(screen.failure, "", "and there is nothing to report")
        screen.destroy()
    }

    function test_the_refused_state_and_the_no_replies_state_are_different() {
        var refused = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": '{"error":"not held"}'
        })
        var empty = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem()])
        })

        verify(refused.readState !== empty.readState,
               "the two must never render as the same screen")
        refused.destroy()
        empty.destroy()
    }

    // ---- an uninterpretable reply is a refusal, never content -------------

    function test_an_unreachable_core_is_a_refusal_not_an_empty_thread() {
        Core.bridge = null
        var screen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", threadId: "root1"
        })

        compare(screen.readState, "failed")
        verify(screen.failure !== "", "and it says why")
        screen.destroy()
    }

    function test_a_reply_that_is_not_json_is_a_refusal() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": "this is not json at all"
        })

        compare(screen.readState, "failed")
        compare(screen.items.length, 0)
        screen.destroy()
    }

    // A success shape carrying neither items nor an error must not render as a
    // thread holding nothing — which is the collapse above arriving by a
    // different route.
    function test_a_success_shape_with_no_items_is_a_refusal() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": '{"page":0,"hasMore":false}'
        })

        compare(screen.readState, "failed",
                "a reply carrying no items is not a thread with no replies")
        screen.destroy()
    }

    // ---- withheld vs. empty ----------------------------------------------

    function test_an_item_with_no_body_field_is_withheld_not_empty() {
        var hiddenRoot = rootItem()
        delete hiddenRoot.body                    // withheld: the key is ABSENT
        hiddenRoot.moderation = { state: "hidden", decidedBy: "mod1" }

        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([hiddenRoot])
        })

        compare(screen.readState, "ok",
                "a withheld body is a moderation outcome, not a failed read")
        verify(screen.items[0].body === undefined,
               "the withheld case is the ABSENCE of the field")
        screen.destroy()
    }

    function test_an_empty_body_is_present_and_not_withheld() {
        var cleared = rootItem()
        cleared.body = { text: "", removed: 0, marked: 0 }   // author cleared it

        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([cleared])
        })

        verify(screen.items[0].body !== undefined,
               "an author who cleared their post reports a body that is present")
        compare(screen.items[0].body.text, "")
        screen.destroy()
    }

    // The distinction stated as the property that matters: the two must be
    // TELLABLE APART by the test the view actually applies. A `body.text || ""`
    // implementation would make these two identical.
    function test_withheld_and_cleared_are_distinguishable_by_the_views_own_test() {
        var withheld = rootItem()
        delete withheld.body
        var cleared = rootItem()
        cleared.id = "r2"
        cleared.parent = "root1"
        cleared.body = { text: "", removed: 0, marked: 0 }

        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([withheld, cleared])
        })

        var withheldIsAbsent = screen.items[0].body === undefined
        var clearedIsAbsent = screen.items[1].body === undefined
        verify(withheldIsAbsent !== clearedIsAbsent,
               "the two must not answer the same to the test the view applies; "
               + "a falsy check on the text would make them identical")
        screen.destroy()
    }

    // ---- moderation is three-valued on a thread item ---------------------
    //
    // A thread item carries NO `isHidden` field — the feed does. Copying a feed
    // delegate here renders blank silently, which is why this is pinned.
    function test_a_thread_item_carries_no_isHidden_field() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem()])
        })

        verify(screen.items[0].isHidden === undefined,
               "a thread item reports moderation as an object, never as isHidden")
        compare(screen.items[0].moderation.state, "unmoderated")
        screen.destroy()
    }

    function test_the_three_moderation_states_are_carried_distinctly() {
        var states = ["unmoderated", "hidden", "unhidden"]
        for (var i = 0; i < states.length; i++) {
            var item = rootItem()
            item.moderation = { state: states[i] }

            var screen = makeScreen({
                "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
                "read_thread": threadOf([item])
            })

            compare(screen.items[0].moderation.state, states[i])
            compare(screen.readState, "ok")
            screen.destroy()
        }
    }

    // ---- the read request ------------------------------------------------

    function test_the_read_names_the_thread_and_the_stoa_it_was_given() {
        var seen = ({})
        Core.bridge = {
            callModule: function (module, method, args) {
                seen[method] = JSON.parse(args[0])
                if (method === "get_capabilities")
                    return '{"canPost":false,"reason":"no keystore"}'
                return JSON.stringify({ items: [rootItem()], page: 0, hasMore: false })
            }
        }

        var screen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", threadId: "root1"
        })

        compare(seen["read_thread"].thread, "root1",
                "the read names the thread it was given")
        compare(seen["read_thread"].stoa, "ab".repeat(32),
                "and the Stoa the feed was rendered for")
        compare(seen["read_thread"].genesis, "00ff",
                "and carries the founding record it was given, unaltered")
        screen.destroy()
    }

    // No thread chosen means no call at all — not a call naming nothing, which
    // core would refuse and the screen would render as a refusal of a question
    // the user never asked.
    function test_no_read_is_made_before_a_thread_is_chosen() {
        var calls = []
        Core.bridge = {
            callModule: function (module, method, args) {
                calls.push(method)
                return '{"canPost":false,"reason":"no keystore"}'
            }
        }

        var screen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", threadId: ""
        })

        compare(calls.indexOf("read_thread"), -1,
                "no thread read is made before a thread has been chosen")
        compare(screen.readState, "failed")
        screen.destroy()
    }
}
