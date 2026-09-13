import QtQuick
import QtTest
import "../src/qml"

// Which screen the app opens on, and the one property that decision must have:
// **it is the module's answer, never a flag the view remembered.**
//
// A remembered "this user has onboarded" outlives the thing it remembers. A
// keystore that was deleted, moved or is unreadable leaves the flag set and the
// user looking at a forum they cannot post in, with no path back to the screen
// that would fix it.
TestCase {
    id: spec
    name: "LaunchBranch"

    property var savedBridge: undefined
    property var calls: []

    // Replies handed out in order, one per who_am_i call. This is how "the
    // second answer decides the branch" is testable at all: the stored state
    // changing between two asks IS two different replies to the same question.
    property var whoAmIQueue: []

    function init() {
        spec.savedBridge = Core.bridge
        spec.calls = []
        spec.whoAmIQueue = []
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    function installBridge(replies) {
        Core.bridge = {
            callModule: function (module, method, args) {
                // `args[0]`, not `String(args)` — the bridge takes an array of
                // JSON strings, and stringifying the array is not parseable.
                var seen = spec.calls.slice()
                seen.push({ method: method, request: String(args[0]) })
                spec.calls = seen

                if (method === "who_am_i" && spec.whoAmIQueue.length > 0) {
                    var n = spec.countOf("who_am_i")
                    var i = Math.min(n - 1, spec.whoAmIQueue.length - 1)
                    return spec.whoAmIQueue[i]
                }
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    function countOf(method) {
        var n = 0
        for (var i = 0; i < spec.calls.length; i++)
            if (spec.calls[i].method === method)
                n++
        return n
    }

    Component {
        id: mainComponent
        Main {}
    }

    // A feed reply, so the "present" branch has something to render rather than
    // failing for an unrelated reason and confusing the diagnosis.
    readonly property var feedReplies: ({
        "get_capabilities": '{"canPost":true,"identity":"aa"}',
        "list_threads": '{"items":[],"page":0,"hasMore":false}'
    })

    function makeMain(replies) {
        installBridge(replies)
        return mainComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            width: 1200,
            height: 900
        })
    }

    // ---- the branch ------------------------------------------------------

    function test_an_identity_reported_present_shows_the_forum() {
        var app = makeMain({
            "who_am_i": '{"hasIdentity":true,"address":"' + "aa".repeat(32)
                      + '","publicKey":"pk","path":3,"recoveryNeedsTheRecord":true}',
            "get_capabilities": spec.feedReplies["get_capabilities"],
            "list_threads": spec.feedReplies["list_threads"]
        })

        compare(app.identityState, "present")
        app.destroy()
    }

    function test_an_identity_reported_absent_shows_onboarding() {
        var app = makeMain({
            "who_am_i": '{"hasIdentity":false,"reason":"no keystore exists at /x/keys"}'
        })

        compare(app.identityState, "absent")
        app.destroy()
    }

    function test_an_unloadable_identity_also_shows_onboarding_with_its_own_reason() {
        // The second absent case: an identity exists but could not be loaded.
        // Core reports `hasIdentity:false` for BOTH, distinguished only by the
        // reason — so both must reach onboarding and the two must stay
        // distinguishable to a later screen.
        var absent = makeMain({
            "who_am_i": '{"hasIdentity":false,"reason":"no keystore exists at /x/keys"}'
        })
        var unreadable = makeMain({
            "who_am_i": '{"hasIdentity":false,"reason":"the keystore at /x/keys could not be read"}'
        })

        compare(absent.identityState, "absent")
        compare(unreadable.identityState, "absent",
                "an identity that cannot be loaded must reach onboarding too")
        verify(absent.identityReason !== unreadable.identityReason,
               "the two absent replies must stay distinguishable by the reason held")
        compare(unreadable.identityReason, "the keystore at /x/keys could not be read",
                "held as the module wrote it, unparsed")
        absent.destroy()
        unreadable.destroy()
    }

    function test_a_failed_report_shows_neither_branch() {
        var app = makeMain({
            "who_am_i": '{"error":"the identity store is locked"}',
            "get_capabilities": spec.feedReplies["get_capabilities"],
            "list_threads": spec.feedReplies["list_threads"]
        })

        compare(app.identityState, "failed")
        verify(app.identityState !== "present", "the forum must not be shown")
        verify(app.identityState !== "absent", "nor a slate")
        compare(app.identityFailure, "the identity store is locked",
                "the module's own message is what is shown")
        app.destroy()
    }

    function test_a_reply_without_hasIdentity_does_not_open_the_forum() {
        // A shape the view cannot read must not be treated as "you are somebody".
        var app = makeMain({ "who_am_i": '{"path":3}' })

        verify(app.identityState !== "present",
               "only an explicit hasIdentity:true may open the forum")
        app.destroy()
    }

    // ---- the branch is re-asked, never remembered ------------------------

    function test_the_second_answer_decides_the_branch_and_the_first_does_not() {
        // The stored state changes from having an identity to having none. A
        // view that remembered the first answer would still be showing a forum.
        spec.whoAmIQueue = [
            '{"hasIdentity":true,"address":"' + "aa".repeat(32)
                + '","publicKey":"pk","path":1,"recoveryNeedsTheRecord":true}',
            '{"hasIdentity":false,"reason":"the keystore was removed"}'
        ]
        var app = makeMain(spec.feedReplies)
        compare(app.identityState, "present", "the fixture's own precondition")

        app.askWhoAmI()

        compare(app.identityState, "absent",
                "the second answer decides; a remembered flag would still say present")
        compare(app.identityReason, "the keystore was removed")
        app.destroy()
    }

    function test_a_keep_moves_the_branch_only_by_asking_the_module_again() {
        // Onboarding says it kept something; the module says there is still
        // nobody. The module wins — an interface that showed a forum because
        // the user pressed keep is right whenever the module is and silent when
        // it is not, which is the failure a user cannot detect.
        spec.whoAmIQueue = [
            '{"hasIdentity":false,"reason":"nothing stored"}',
            '{"hasIdentity":false,"reason":"nothing stored, still"}'
        ]
        var app = makeMain(spec.feedReplies)
        compare(app.identityState, "absent")
        var before = spec.countOf("who_am_i")

        // The real wiring: the screen's own signal, not a test-only hook.
        var screen = spec.onboardingOn(app)
        verify(screen !== null, "onboarding must actually be on screen")
        screen.identityKept()

        compare(spec.countOf("who_am_i"), before + 1,
                "the keep must cause the module to be asked again")
        compare(app.identityState, "absent",
                "and the module's answer decides, not the keep")
        app.destroy()
    }

    function test_a_generated_and_selected_candidate_is_never_an_identity() {
        // A candidate that reached the forum as "you" would attribute a user's
        // reading, and eventually their posting attempts, to a key that exists
        // nowhere.
        spec.whoAmIQueue = [
            '{"hasIdentity":false,"reason":"nothing stored"}',
            '{"hasIdentity":false,"reason":"nothing stored"}'
        ]
        var app = makeMain({
            "generate_identity_slate":
                '{"slate":"s1","count":1,"candidates":[{"index":0,"path":0,"address":"'
                + "55".repeat(32) + '","publicKey":"pk"}]}'
        })
        compare(app.identityState, "absent")

        var screen = spec.onboardingOn(app)
        verify(screen !== null, "onboarding must actually be on screen")
        screen.requestSlate()
        screen.select(0)
        compare(screen.selectedIndex, 0, "the fixture's own precondition")

        // The launch decision taken again, mid-flow.
        app.askWhoAmI()

        compare(app.identityState, "absent",
               "a module reporting no identity shows onboarding, whatever the "
               + "screen is holding")
        compare(screen.keptIdentity, null, "and no candidate became an identity")
        app.destroy()
    }

    function test_the_recovery_flag_reaches_the_screen_from_the_report() {
        var app = makeMain({
            "who_am_i": '{"hasIdentity":false,"reason":"none","recoveryNeedsTheRecord":true}'
        })
        var screen = spec.onboardingOn(app)
        verify(screen !== null)
        compare(screen.recoveryNeedsTheRecord, true,
                "what the module reported is what the screen holds")
        app.destroy()
    }

    function test_an_omitted_recovery_flag_reaches_the_screen_as_no_claim() {
        var app = makeMain({
            "who_am_i": '{"hasIdentity":false,"reason":"none"}'
        })
        var screen = spec.onboardingOn(app)
        verify(screen !== null)
        compare(screen.recoveryNeedsTheRecord, undefined,
                "a field the module omitted must not become a reported false")
        app.destroy()
    }

    // ---- finding the onboarding screen in the tree ----------------------

    function onboardingOn(item) {
        if (item === null || item === undefined)
            return null
        // Duck-typed on the properties only OnboardingScreen has, because
        // `instanceof` over a QML type is not available here.
        if (item.phase !== undefined && item.candidates !== undefined
            && item.keptIdentity !== undefined)
            return item
        var kids = item.children
        if (kids === undefined)
            return null
        for (var i = 0; i < kids.length; i++) {
            var hit = spec.onboardingOn(kids[i])
            if (hit !== null)
                return hit
        }
        return null
    }
}
