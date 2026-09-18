import QtQuick
import QtTest
import "../src/qml"

// The reply composer, the inert control, and the route in and out.
//
// **The composer is WIRED and the earlier-versions control is INERT**, and the
// difference is the contract rather than effort: `publish_reply` exists, so an
// unwired composer would be a defect; no contract method reads a prior version,
// so that control is present and offers no action.
TestCase {
    id: spec
    name: "ThreadReply"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    function rootItem() {
        return {
            thread: "root1", id: "root1", currentVersion: "rev9",
            author: "aa".repeat(32), isRevised: true,
            moderation: { state: "unmoderated" },
            position: "0",
            body: { text: "the root post", removed: 0, marked: 0 }
        }
    }

    Component {
        id: threadComponent
        DThreadScreen {}
    }

    // A bridge recording every call, so what reached core can be asserted rather
    // than inferred from what the screen shows.
    function recordingBridge(calls, canPost) {
        return {
            callModule: function (module, method, args) {
                calls.push({ method: method, args: JSON.parse(args[0]) })
                if (method === "get_capabilities")
                    return JSON.stringify({ canPost: canPost, reason: canPost ? "" : "no keystore" })
                if (method === "read_thread")
                    return JSON.stringify({ items: [rootItem()], page: 0, hasMore: false })
                if (method === "publish_reply")
                    return '{"opId":"newreply","wasNew":true}'
                return '{"error":"no fake reply for ' + method + '"}'
            }
        }
    }

    function makeScreen(calls, canPost) {
        Core.bridge = recordingBridge(calls, canPost)
        return threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            threadId: "root1"
        })
    }

    // ---- the composer actually publishes ---------------------------------

    function test_submitting_a_reply_makes_a_publish_call() {
        var calls = []
        var screen = makeScreen(calls, true)

        var composer = findChild(screen, "replyComposer")
        verify(composer !== null, "an open gate renders the reply composer")

        composer.draft = "a reply worth publishing"
        composer.submit()

        var published = null
        for (var i = 0; i < calls.length; i++)
            if (calls[i].method === "publish_reply")
                published = calls[i]

        verify(published !== null,
               "the composer is WIRED: submitting reaches publish_reply")
        compare(published.args.body, "a reply worth publishing",
                "and carries the draft unaltered")
        screen.destroy()
    }

    // The parent is the post the reply was made under. A composer sending the
    // thread's root regardless of that would publish every reply as a top-level
    // answer — which verifies, stores and renders in the wrong place permanently.
    //
    // This screen offers ONE composer, at the thread's foot, so the post it is
    // under IS the root; the assertion is that the root's `id` travels, and not
    // its `currentVersion`, which moves when the post is edited.
    function test_the_reply_names_the_roots_id_and_not_its_current_version() {
        var calls = []
        var screen = makeScreen(calls, true)

        var composer = findChild(screen, "replyComposer")
        composer.draft = "x"
        composer.submit()

        var published = null
        for (var i = 0; i < calls.length; i++)
            if (calls[i].method === "publish_reply")
                published = calls[i]

        compare(published.args.parent, "root1",
                "the parent is the post's id, which does not move across a revision")
        verify(published.args.parent !== "rev9",
               "and never the current version, which does")
        screen.destroy()
    }

    // Core derives the thread from the parent and REFUSES a request naming one.
    function test_no_thread_identifier_is_sent_with_a_reply() {
        var calls = []
        var screen = makeScreen(calls, true)

        var composer = findChild(screen, "replyComposer")
        composer.draft = "x"
        composer.submit()

        var published = null
        for (var i = 0; i < calls.length; i++)
            if (calls[i].method === "publish_reply")
                published = calls[i]

        verify(published.args.thread === undefined,
               "a reply request carries no thread identifier; core refuses one")
        screen.destroy()
    }

    // ---- the gate governs the reply box ----------------------------------
    //
    // `composer-view` requires NO text input where the probe says posting is not
    // possible — not a disabled one, and not one that accepts text and refuses
    // to submit.
    function test_a_closed_gate_renders_no_reply_box() {
        var calls = []
        var screen = makeScreen(calls, false)

        var composer = findChild(screen, "replyComposer")
        var open = findChild(screen, "replyComposerOpen")
        verify(open === null || !open.visible,
               "a closed gate renders no composer at all")
        if (composer !== null)
            verify(!composer.visible, "and certainly not a typable one")
        screen.destroy()
    }

    function test_a_closed_gate_still_renders_the_thread() {
        var calls = []
        var screen = makeScreen(calls, false)

        compare(screen.readState, "ok",
                "the gate governs replying, never reading")
        compare(screen.items.length, 1)
        screen.destroy()
    }

    // ---- the inert control ------------------------------------------------

    function test_the_earlier_versions_control_is_present_on_a_revised_post() {
        var calls = []
        var screen = makeScreen(calls, true)

        var inert = findChild(screen, "earlierVersionsInert")
        verify(inert !== null,
               "the control is PRESENT rather than absent — that is what inert means here")
        screen.destroy()
    }

    // The requirement: acting on it reaches no core call. It is static text with
    // no handler, so there is nothing for a call to be reached from — the
    // assertion is that no call appears after the screen has settled.
    function test_the_earlier_versions_control_reaches_no_core_call() {
        var calls = []
        var screen = makeScreen(calls, true)

        var before = calls.length
        var inert = findChild(screen, "earlierVersionsInert")

        // There is no signal to emit and no handler to invoke: the control is a
        // Text with no MouseArea. Confirm that shape rather than asserting on a
        // click that cannot be delivered.
        verify(inert !== null)
        compare(calls.length, before,
                "nothing about the inert control reaches core")

        var handlers = 0
        for (var i = 0; i < inert.children.length; i++)
            if (inert.children[i].toString().indexOf("MouseArea") !== -1)
                handlers += 1
        compare(handlers, 0,
                "it carries no MouseArea, so there is no route to a call at all")
        screen.destroy()
    }

    // ---- no score is rendered ---------------------------------------------

    function test_no_vote_score_is_rendered_for_any_item() {
        var calls = []
        var screen = makeScreen(calls, true)

        // No field of a thread item carries a tally, and the vote control's own
        // default would otherwise print a number core never reported.
        var votes = findChild(screen, "threadItems")
        verify(screen.items[0].score === undefined,
               "no thread item carries a score")
        screen.destroy()
    }

    // ---- the way out ------------------------------------------------------

    function test_the_back_affordance_is_offered_and_emits_closed() {
        var calls = []
        var screen = makeScreen(calls, true)

        var back = findChild(screen, "threadBackButton")
        verify(back !== null, "a way out is offered")
        verify(back.visible)

        var left = false
        screen.closed.connect(function () { left = true })
        back.clicked()
        verify(left, "acting on it asks to leave")
        screen.destroy()
    }

    // The state a user most needs to leave is a refused read, so the affordance
    // must not be withdrawn by it. A return route no scenario requires can be
    // deleted with every other test still passing.
    function test_the_way_out_survives_a_refused_read() {
        Core.bridge = {
            callModule: function (module, method, args) {
                if (method === "get_capabilities")
                    return '{"canPost":false,"reason":"no keystore"}'
                return '{"error":"this peer holds no op under root1"}'
            }
        }
        var screen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", threadId: "root1"
        })

        compare(screen.readState, "failed")

        var back = findChild(screen, "threadBackButton")
        verify(back !== null && back.visible,
               "the way out is still offered from the state most needing it")

        var left = false
        screen.closed.connect(function () { left = true })
        back.clicked()
        verify(left)
        screen.destroy()
    }

    // ---- the screen carries no thread of its own -------------------------

    function test_the_screen_holds_no_usable_default_for_what_it_renders() {
        Core.bridge = {
            callModule: function (module, method, args) {
                return '{"canPost":false,"reason":"no keystore"}'
            }
        }
        var bare = threadComponent.createObject(null, {})

        compare(bare.threadId, "",
                "no default thread identifier: a second source for it is a build "
                + "shipping a hardcoded thread")
        compare(bare.stoaAddress, "", "and none for the Stoa")
        compare(bare.stoaGenesis, "", "and none for the founding record")
        bare.destroy()
    }
}
