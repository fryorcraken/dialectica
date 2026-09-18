import QtQuick
import QtTest
import "../src/qml"

// The navigator's transitions: which screen is shown, what travels across each
// move, and that every state a user can enter can be left.
//
// **These tests drive `Main.qml`, not a screen.** That is the point of the file:
// a component suite instantiates the component it tests and therefore supplies
// the reachability whose absence was the defect — 92 test functions passed over
// four screens no user could open. Driving the root is what makes "the user got
// there" a thing a test can observe at all.
//
// It still cannot discharge the reachability REQUIREMENT, and does not claim to.
// A type this file forgets to look for is indistinguishable from one nobody
// thought of; enumerating the registrations is `check_qml_reachable.py`'s, and
// that gate reads `qmldir` rather than asserting against a tree it built.
TestCase {
    id: spec
    name: "Navigation"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    // The house fake: answers each method from a map and RECORDS every call.
    //
    // The recording half is what lets a test assert a call was NOT made, which
    // several assertions below turn on — "returned without keeping anything" and
    // "made no keep call" are the same claim, and only the call log shows it.
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

    // Visibility checked on every ancestor, because QML's `visible` is not
    // inherited into the property: an element inside a hidden panel still
    // reports `visible: true` for itself, and an absence assertion that missed
    // that would pass on a screen showing the affordance.
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

    Component { id: mainComponent; Main {} }

    readonly property string stoaA:
        "aaaaaaaa11111111111111111111111111111111111111111111111111111111"
    readonly property string keyA:
        "k:0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"

    // A listing holding one Stoa, so the list screen has a row to open.
    readonly property string oneStoa:
        '{"items":[{"stoa":"' + stoaA + '","foundingTitle":"Nym Research"}],'
        + '"page":0,"hasMore":false}'

    // ---- the routing state reads BOTH probes ----------------------------

    // The load-bearing case. `who_am_i` says there IS an identity and
    // `get_capabilities` says posting is not possible — the two honestly
    // disagreeing, which `lib.rs:258` contracts they can. Routing on the posting
    // probe alone would collapse this into "there is nobody here" and offer key
    // CREATION, which for an existing identity is irreversible.
    function test_an_unusable_identity_is_not_offered_identity_creation() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":true,"publicKey":"' + spec.keyA + '","path":0}',
            "get_capabilities": '{"canPost":false,"reason":"the keystore is readable by others"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")

        var feed = spec.namedAnywhere(view, "feed")[0]
        compare(feed.hasIdentity, true,
                "who_am_i reported an identity, so the view has one")
        compare(feed.capability.canPost, false,
                "and the posting probe says it cannot currently be used")

        // The chip is bound to the identity report, so it renders the
        // identity-present arm and offers no creation route.
        var chip = spec.namedAnywhere(view, "identityChip")[0]
        compare(chip.hasIdentity, true,
                "the chip must not offer key creation to a user who HAS a key — "
                + "keeping is refused where an identity exists, and replacing "
                + "one discards every identity derived from it")
        view.destroy()
    }

    function test_no_identity_at_all_does_offer_identity_creation() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"no keystore on this machine"}',
            "get_capabilities": '{"canPost":false,"reason":"no keystore on this machine"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")

        var chip = spec.namedAnywhere(view, "identityChip")[0]
        compare(chip.hasIdentity, false)
        view.destroy()
    }

    // Both probes, every render, neither answered from a retained value.
    function test_both_probes_are_re_read_on_every_render() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")

        var feed = spec.namedAnywhere(view, "feed")[0]
        var whoFirst = spec.callsTo(bridge, "who_am_i")
        var capFirst = spec.callsTo(bridge, "get_capabilities")
        verify(whoFirst > 0, "the identity report was asked at least once")
        verify(capFirst > 0, "the posting probe was asked at least once")

        feed.reload()

        compare(spec.callsTo(bridge, "who_am_i"), whoFirst + 1,
                "the identity report is asked again rather than answered from "
                + "the earlier render — a retained answer outlives the keystore "
                + "it stands for")
        compare(spec.callsTo(bridge, "get_capabilities"), capFirst + 1,
                "and so is the posting probe")
        view.destroy()
    }

    // A probe answering something that is not `true` must not read as an
    // identity. `"true"`, `1` and `null` are each truthy-or-falsy in a way that
    // does not match what they mean.
    function test_a_degenerate_identity_reply_claims_no_identity() {
        var shapes = ['{"hasIdentity":"true"}', '{"hasIdentity":1}',
                      '{"hasIdentity":null}', '{}']
        for (var i = 0; i < shapes.length; i++) {
            Core.bridge = spec.bridgeFor({
                "list_stoas": spec.oneStoa,
                "list_threads": '{"items":[],"page":0,"hasMore":false}',
                "who_am_i": shapes[i],
                "get_capabilities": '{"canPost":false,"reason":"none"}'
            })
            var view = mainComponent.createObject(null, {})
            view.open(spec.stoaA, "Nym Research", "")
            var feed = spec.namedAnywhere(view, "feed")[0]
            compare(feed.hasIdentity, false,
                    "reply " + shapes[i] + " must not claim an identity")
            view.destroy()
        }
    }

    // ---- onboarding: in, and out both ways ------------------------------

    function test_the_create_affordance_reaches_the_onboarding_screen() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")
        compare(view.screenShown, "feed")

        var chip = spec.namedAnywhere(view, "identityChip")[0]
        chip.createRequested()

        compare(view.screenShown, "onboarding",
                "acting on the affordance renders the screen where an identity "
                + "is acquired")
        var screen = spec.namedAnywhere(view, "onboarding")[0]
        compare(screen.stoaAddress, spec.stoaA,
                "and it is given the Stoa the identity is being chosen for — "
                + "every core call on that screen takes one")
        view.destroy()
    }

    // The route out is offered in EVERY phase, including the opening one where
    // nothing has been asked for yet. A route out offered only on success is a
    // route absent in exactly the cases where the user is stuck.
    function test_the_screen_can_be_left_without_keeping_anything() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")
        view.createIdentityFor(spec.stoaA, "Nym Research", "")

        var back = spec.visibleNamed(view, "onboardingBackButton")
        compare(back.length, 1, "a way out is offered before anything is kept")

        back[0].clicked()

        compare(view.screenShown, "feed", "and it returns to the feed")
        compare(spec.callsTo(bridge, "keep_identity"), 0,
                "leaving keeps nothing")
        view.destroy()
    }

    // A keep that came back refused, and one that failed. In each case the user
    // is in a state they entered and must be able to leave it.
    function test_a_refused_or_failed_keep_still_leaves_a_way_out() {
        var outcomes = [
            { keep: '{"kept":false,"reason":"an identity already exists"}',
              phase: "refused" },
            { keep: '{"error":"the keystore could not be written"}',
              phase: "failed" }
        ]
        for (var i = 0; i < outcomes.length; i++) {
            Core.bridge = spec.bridgeFor({
                "list_stoas": spec.oneStoa,
                "list_threads": '{"items":[],"page":0,"hasMore":false}',
                "who_am_i": '{"hasIdentity":false,"reason":"none"}',
                "get_capabilities": '{"canPost":false,"reason":"none"}',
                "generate_identity_slate":
                    '{"slate":"ab","count":1,"candidates":[{"index":0,"publicKey":"'
                    + spec.keyA + '"}]}',
                "keep_identity": outcomes[i].keep
            })
            var view = mainComponent.createObject(null, {})
            view.createIdentityFor(spec.stoaA, "Nym Research", "")

            var screen = spec.namedAnywhere(view, "onboarding")[0]
            screen.requestSlate()
            screen.select(0)
            screen.keepSelected()
            compare(screen.phase, outcomes[i].phase,
                    "the keep came back as a " + outcomes[i].phase)

            compare(spec.visibleNamed(view, "onboardingBackButton").length, 1,
                    "a way out is still offered after a " + outcomes[i].phase
                    + " keep — this is exactly when the user is stuck")
            view.destroy()
        }
    }

    // The keep signal carries no identity by design, so the navigator must
    // re-ask the module rather than treating the signal as the answer.
    function test_a_kept_identity_returns_to_the_feed_and_re_asks_who_i_am() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":true,"publicKey":"' + spec.keyA + '","path":0}',
            "get_capabilities": '{"canPost":true}',
            "generate_identity_slate":
                '{"slate":"ab","count":1,"candidates":[{"index":0,"publicKey":"'
                + spec.keyA + '"}]}',
            "keep_identity": '{"kept":true,"publicKey":"' + spec.keyA
                             + '","path":0,"encrypted":true}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        view.createIdentityFor(spec.stoaA, "Nym Research", "")

        var before = spec.callsTo(bridge, "who_am_i")
        var screen = spec.namedAnywhere(view, "onboarding")[0]
        screen.requestSlate()
        screen.select(0)
        screen.keepSelected()

        compare(view.screenShown, "feed",
                "a kept identity returns the user to a screen they can act on")
        verify(spec.callsTo(bridge, "who_am_i") > before,
               "and the identity shown there comes from a FRESH identity "
               + "report, not from the keep signal — which carries no identity")
        view.destroy()
    }

    // ---- the thread route -----------------------------------------------

    function test_a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op() {
        var op = "cc" + "11".repeat(31)
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[{"currentVersion":"' + op
                + '","author":"' + spec.keyA + '","body":{"text":"hi"}}],'
                + '"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")

        var link = spec.visibleNamed(view, "readThreadLink")
        compare(link.length, 1, "the row offers a route into its thread")

        var feed = spec.namedAnywhere(view, "feed")[0]
        feed.threadOpened(op)

        compare(view.screenShown, "thread")
        var args = String(spec.lastArgsTo(bridge, "read_thread"))
        verify(args.indexOf('"stoa":"' + spec.stoaA + '"') >= 0,
               "the thread is given the row's Stoa")
        verify(args.indexOf('"thread":"' + op + '"') >= 0,
               "and the ROOT POST's identifier, which does not move when the "
               + "post is edited")
        verify(args.indexOf('"genesis":"beef"') >= 0,
               "and the founding record the view holds for that Stoa")
        view.destroy()
    }

    // Where the view holds no record, the thread is opened WITHOUT one rather
    // than with a fabricated or placeholder one. A fabricated record fails
    // verification in the core and surfaces as a refusal the user cannot act on.
    function test_no_record_is_invented_for_a_stoa_the_view_has_none_for() {
        var op = "cc" + "11".repeat(31)
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        // Opened with NO genesis — the listing does not return one.
        view.open(spec.stoaA, "Nym Research", "")

        var feed = spec.namedAnywhere(view, "feed")[0]
        feed.threadOpened(op)

        var args = String(spec.lastArgsTo(bridge, "read_thread"))
        verify(args.indexOf('"genesis":""') >= 0,
               "the empty record travels as empty — the view neither invents "
               + "one nor substitutes a placeholder. What core then refuses is "
               + "rendered as the refusal it is: " + args)
        view.destroy()
    }

    function test_the_feed_is_reached_again_from_the_thread() {
        var op = "cc" + "11".repeat(31)
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")
        var feed = spec.namedAnywhere(view, "feed")[0]
        feed.threadOpened(op)
        compare(view.screenShown, "thread")

        var back = spec.visibleNamed(view, "threadBackButton")
        compare(back.length, 1, "the thread offers a way back to the feed")
        back[0].clicked()

        compare(view.screenShown, "feed")
        compare(feed.stoaAddress, spec.stoaA,
                "and it is the feed for the Stoa the thread was opened from, "
                + "without the view being restarted")
        view.destroy()
    }

    // The way out of a thread whose read FAILED. That is when a user most wants
    // out, so the control must not be withdrawn by the state it exists to leave.
    function test_a_thread_that_could_not_be_read_still_offers_the_way_back() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"error":"no root held for that thread"}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")
        spec.namedAnywhere(view, "feed")[0].threadOpened("cc" + "11".repeat(31))

        var thread = spec.namedAnywhere(view, "thread")[0]
        compare(thread.readState, "failed")
        compare(spec.visibleNamed(view, "threadBackButton").length, 1,
                "a failed read is exactly when the way out must be there")
        view.destroy()
    }

    // ---- exactly one screen, from one source -----------------------------

    function test_exactly_one_screen_is_shown_in_every_reachable_state() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        var names = ["stoaList", "joinScreen", "feed", "thread", "onboarding"]

        function shownCount() {
            var n = 0
            for (var i = 0; i < names.length; i++)
                n += spec.visibleNamed(view, names[i]).length
            return n
        }

        compare(shownCount(), 1, "the list alone, at startup")

        view.preview(spec.stoaA, "beef")
        compare(shownCount(), 1, "the join preview alone")

        view.open(spec.stoaA, "Nym Research", "beef")
        compare(shownCount(), 1, "the feed alone")

        view.openThread("cc" + "11".repeat(31))
        compare(shownCount(), 1, "the thread alone")

        view.createIdentityFor(spec.stoaA, "Nym Research", "beef")
        compare(shownCount(), 1, "onboarding alone")
        view.destroy()
    }

    // Each transition clears what the others own, so the state in which two are
    // set — which has no rendering, and which an ordered ternary would resolve
    // by accident of which test came first — cannot be constructed.
    function test_no_transition_leaves_two_states_set_at_once() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})

        function setCount() {
            return (view.chosen !== null ? 1 : 0)
                 + (view.previewing !== null ? 1 : 0)
                 + (view.reading !== null ? 1 : 0)
                 + (view.onboarding !== null ? 1 : 0)
        }

        view.preview(spec.stoaA, "beef");            compare(setCount(), 1)
        view.open(spec.stoaA, "Nym Research", "beef"); compare(setCount(), 1)
        view.openThread("cc" + "11".repeat(31));     compare(setCount(), 1)
        view.closeThread();                          compare(setCount(), 1)
        view.createIdentityFor(spec.stoaA, "T", ""); compare(setCount(), 1)
        view.closeOnboarding();                      compare(setCount(), 1)
        view.closeFeed();                            compare(setCount(), 0)
        view.destroy()
    }

    // ---- shared chrome ---------------------------------------------------

    // The lamps accompany EVERY main-area screen. A status indicator absent from
    // a screen is one whose absence a user reads as "nothing to report".
    function test_the_status_bar_accompanies_every_main_area_screen() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})

        var states = [
            function () { view.closeFeed() },
            function () { view.preview(spec.stoaA, "beef") },
            function () { view.open(spec.stoaA, "Nym Research", "beef") },
            function () { view.openThread("cc" + "11".repeat(31)) },
            function () { view.createIdentityFor(spec.stoaA, "T", "") }
        ]
        for (var i = 0; i < states.length; i++) {
            states[i]()
            compare(spec.visibleNamed(view, "statusBar").length, 1,
                    "the lamps are rendered alongside " + view.screenShown)
        }
        view.destroy()
    }

    // Unbound chrome must not report health. Green is a claim the software
    // cannot back before the values have arrived, and every screen passes
    // through that state between appearing and core answering.
    function test_an_unbound_lamp_does_not_report_health() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": '{"error":"the store could not be read"}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        var bar = spec.namedAnywhere(view, "statusBar")[0]

        // NO SPEC: the spec requires the least-claiming appearance for an
        // unbound value and does not name which string that is. `DStatusBar`
        // already answers "degraded" for both an unset and an unrecognised
        // state, so the delivery lamp — deliberately bound to nothing, because
        // no synchronous call produces a delivery outcome — reads degraded.
        compare(bar.normalisedState(bar.deliveryState), "degraded",
                "the delivery lamp has no honest source and must not claim one")
        verify(bar.normalisedState(bar.storageState) !== "ok",
               "a store that could not be read is not reported as healthy")
        view.destroy()
    }
}
