import QtQuick
import QtTest
import "../src/qml"

// The onboarding flow's states, and the three distinctions the screen exists to
// make:
//
//   * a refused keep is neither a success nor a failure;
//   * nothing is selected until the user selects it;
//   * a reply the screen cannot read is a failure rather than an empty offer.
//
// These drive the real OnboardingScreen through a fake bridge, so what is under
// test is the screen's own state machine rather than a re-implementation of it.
//
// **Every assertion is against a value the fixture chose**, not against
// whatever the screen produced: the addresses, reasons and flags below are
// written into the fake replies and then compared to what the screen holds. A
// test that asked the screen what it wrote and agreed would pass against any
// implementation at all.
TestCase {
    id: spec
    name: "OnboardingStates"

    property var savedBridge: undefined

    // Every call the fake bridge saw, in order: `{ method, args }`. Several
    // requirements are about calls that must NOT happen (no slate on arrival,
    // no call while selecting, no keep while nothing is selected), and a log is
    // the only way to assert on the absence of one.
    property var calls: []

    function init() {
        spec.savedBridge = Core.bridge
        spec.calls = []
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                // Push a copy rather than mutate in place: QML's var property
                // does not notify on `push`, and a later read of a mutated
                // array has bitten this kind of harness before.
                // The first element, not `String(args)`: the bridge takes an
                // ARRAY of JSON strings, and stringifying the array itself
                // yields `[object Object]`-adjacent junk that JSON.parse then
                // rejects. Keeping the element is what makes the request
                // assertable as a parsed object rather than by substring.
                var seen = spec.calls.slice()
                seen.push({ method: method, request: String(args[0]) })
                spec.calls = seen

                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                // An array lets one method answer differently on successive
                // calls, which the refresh tests need.
                if (Array.isArray(replies[method])) {
                    var queue = replies[method]
                    var i = Math.min(spec.countOf(method) - 1, queue.length - 1)
                    return queue[i]
                }
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

    function makeScreen(replies) {
        Core.bridge = bridgeFor(replies)
        return onboardingComponent.createObject(null, {
            stoaAddress: "ab".repeat(32)
        })
    }

    Component {
        id: onboardingComponent
        OnboardingScreen {}
    }

    // Two candidates whose addresses differ in every character, so "the two
    // rows show different address text" cannot pass by accident.
    readonly property string addrA: "11".repeat(32)
    readonly property string addrB: "22".repeat(32)

    readonly property string twoCandidateSlate:
        '{"slate":"feed01","count":2,"candidates":['
        + '{"index":0,"path":0,"address":"' + "11".repeat(32) + '","publicKey":"aa"},'
        + '{"index":1,"path":1,"address":"' + "22".repeat(32) + '","publicKey":"bb"}'
        + ']}'

    // ---- the opening state ----------------------------------------------

    function test_the_opening_state_holds_nothing_and_has_called_nothing() {
        var screen = makeScreen({})

        compare(screen.phase, "intro")
        compare(screen.candidates.length, 0, "the opening state holds no candidates")
        compare(spec.countOf("generate_identity_slate"), 0,
                "a slate generated on arrival is a set the user never asked to see")
        compare(spec.countOf("keep_identity"), 0)
        screen.destroy()
    }

    function test_requesting_candidates_is_what_calls_the_module() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })

        screen.requestSlate()

        compare(spec.countOf("generate_identity_slate"), 1,
                "exactly one slate call, and only once asked for")
        compare(screen.phase, "slate")
        screen.destroy()
    }

    // ---- the slate ------------------------------------------------------

    function test_every_candidate_gets_a_row_in_the_replys_order() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        compare(screen.candidates.length, 2)
        // Against the addresses THIS TEST wrote into the reply, in the order it
        // wrote them — not against whatever order the screen happened to hold.
        compare(screen.candidates[0].address, spec.addrA, "order must be the reply's")
        compare(screen.candidates[1].address, spec.addrB)
        verify(screen.candidates[0].address !== screen.candidates[1].address,
               "two candidates must be distinguishable by their address text")
        screen.destroy()
    }

    function test_the_count_comes_from_the_reply_not_from_a_number_in_the_view() {
        // THREE candidates, which is neither the five core ships nor the two
        // every other fixture here uses. A view with a hardcoded count passes
        // the two-candidate tests and fails this one.
        var screen = makeScreen({
            "generate_identity_slate":
                '{"slate":"s3","count":3,"candidates":['
                + '{"index":0,"path":0,"address":"aa","publicKey":"p0"},'
                + '{"index":1,"path":1,"address":"bb","publicKey":"p1"},'
                + '{"index":2,"path":2,"address":"cc","publicKey":"p2"}]}'
        })
        screen.requestSlate()

        compare(screen.candidates.length, 3,
                "the number of rows is the reply's, not the view's")
        screen.destroy()
    }

    function test_a_reply_with_no_candidate_list_is_a_failure_not_an_empty_slate() {
        // The distinction this screen shares with the feed: an unreadable
        // answer and an empty one mean opposite things and look identical when
        // collapsed.
        var noList = makeScreen({
            "generate_identity_slate": '{"slate":"s1","count":0}'
        })
        noList.requestSlate()
        compare(noList.phase, "failed",
                "a reply carrying no candidate list must not render as an empty offer")
        verify(noList.failure.length > 0, "the failure must name itself")
        noList.destroy()

        // Present but not a list — passes an `=== undefined` check alone and
        // then has `.length` read off it.
        var notArray = makeScreen({
            "generate_identity_slate": '{"slate":"s1","count":2,"candidates":"two"}'
        })
        notArray.requestSlate()
        compare(notArray.phase, "failed", "candidates must be an ARRAY, not merely present")
        notArray.destroy()
    }

    function test_a_failed_slate_call_shows_the_modules_own_message() {
        var screen = makeScreen({
            "generate_identity_slate": '{"error":"the keystore at /x/keys is not readable"}'
        })
        screen.requestSlate()

        compare(screen.phase, "failed")
        // The exact string the fixture put on the wire, so a reworded or
        // summarised message fails rather than an empty one.
        compare(screen.failure, "the keystore at /x/keys is not readable",
                "core's message names a fix and must not be reworded by the view")
        screen.destroy()
    }

    // ---- refreshing ------------------------------------------------------

    function test_repeated_requests_are_each_answered_and_never_rationed() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })

        for (var i = 0; i < 12; i++)
            screen.requestSlate()

        compare(spec.countOf("generate_identity_slate"), 12,
                "regeneration is unlimited by contract; the view must ration nothing")
        compare(screen.phase, "slate", "the twelfth request was served, not swallowed")
        screen.destroy()
    }

    function test_a_new_set_replaces_the_old_candidates_entirely() {
        var screen = makeScreen({
            "generate_identity_slate": [
                spec.twoCandidateSlate,
                '{"slate":"feed02","count":1,"candidates":['
                + '{"index":0,"path":9,"address":"' + "33".repeat(32) + '","publicKey":"cc"}]}'
            ]
        })

        screen.requestSlate()
        compare(screen.candidates[0].address, spec.addrA)

        screen.requestSlate()
        compare(screen.candidates.length, 1, "only the second set is held")
        compare(screen.candidates[0].address, "33".repeat(32))
        for (var i = 0; i < screen.candidates.length; i++) {
            verify(screen.candidates[i].address !== spec.addrA,
                   "no candidate of the first set may still be on screen")
            verify(screen.candidates[i].address !== spec.addrB)
        }
        compare(screen.slateId, "feed02", "the set identifier is the new set's")
        screen.destroy()
    }

    function test_a_refresh_clears_the_selection() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()
        screen.select(1)
        compare(screen.selectedIndex, 1, "the fixture's own precondition")

        screen.requestSlate()

        compare(screen.selectedIndex, -1,
                "a position carried across a refresh names a candidate never looked at")
        screen.destroy()
    }

    function test_a_failed_refresh_does_not_leave_a_stale_set_on_screen() {
        var screen = makeScreen({
            "generate_identity_slate": [
                spec.twoCandidateSlate,
                '{"error":"the store went away"}'
            ]
        })
        screen.requestSlate()
        screen.select(0)

        screen.requestSlate()

        compare(screen.phase, "failed")
        compare(screen.candidates.length, 0,
                "a failure must not leave the previous set on screen as current")
        compare(screen.selectedIndex, -1)
        screen.destroy()
    }

    // ---- selection -------------------------------------------------------

    // Regression, correctness review: `-1` was both the "nothing selected"
    // sentinel and a value a reply's `index` could carry, so a candidate
    // carrying `index:-1` drew the chosen background, the chosen border and a
    // visible SELECTED word while `selectedIndex` was `-1` and nothing was
    // selected. Pressing Keep then hit the `selectedIndex < 0` guard and
    // returned silently, and clicking the row called `select(-1)` which changed
    // nothing — a dead button with no way for the user to tell why.
    //
    // Asserted on what an OBSERVER OF THE SCREEN can see, not on the sentinel's
    // value: a test comparing `selectedIndex` to `-1` passed throughout the
    // defect, because `selectedIndex` was never wrong. What was wrong was the
    // row.
    readonly property string slateWithNegativeIndex:
        '{"slate":"s1","count":2,"candidates":['
        + '{"index":-1,"path":0,"address":"' + "11".repeat(32) + '","publicKey":"aa"},'
        + '{"index":0,"path":1,"address":"' + "22".repeat(32) + '","publicKey":"bb"}'
        + ']}'

    function test_no_row_reads_as_chosen_while_nothing_is_selected() {
        var normal = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        normal.requestSlate()
        compare(spec.visibleSelectedMarkers(normal), 0,
                "the fixture's own precondition: a normal slate marks no row")
        normal.destroy()

        var hostile = makeScreen({ "generate_identity_slate": spec.slateWithNegativeIndex })
        hostile.requestSlate()

        compare(hostile.selectedIndex, -1, "nothing is selected")
        compare(spec.visibleSelectedMarkers(hostile), 0,
                "so NO row may read as chosen — a row drawing SELECTED while "
                + "nothing is selected is a choice made for the user, and the "
                + "keep button is dead beneath it")
        hostile.destroy()
    }

    function test_a_candidate_whose_index_is_not_a_position_is_a_failure() {
        // The durable fix is that a candidate list containing an entry the
        // screen cannot address is not a slate. A range check that let the row
        // render and merely refused to select it would leave the user looking
        // at a candidate they cannot choose.
        var screen = makeScreen({ "generate_identity_slate": spec.slateWithNegativeIndex })
        screen.requestSlate()

        compare(screen.phase, "failed",
                "a candidate the screen cannot address is a reply it cannot read")
        compare(screen.candidates.length, 0)
        verify(screen.failure.length > 0, "the failure must name itself")
        screen.destroy()
    }

    function test_a_candidate_that_is_not_an_object_is_a_failure_not_a_blank_row() {
        // Regression, correctness review: `Array.isArray` established that
        // `candidates` was a list, not that its ENTRIES were candidates. A
        // reply of `[null,"str",{...}]` reached the slate phase and built three
        // rows, one of them blank, with the delegate throwing on every access —
        // and the user was invited to choose between them.
        var screen = makeScreen({
            "generate_identity_slate":
                '{"slate":"s1","count":3,"candidates":[null,"str",{"index":0,"path":0,'
                + '"address":"' + "11".repeat(32) + '","publicKey":"aa"}]}'
        })
        screen.requestSlate()

        compare(screen.phase, "failed",
                "an array whose elements are unreadable is the same confusion "
                + "one level down: a blank row is a candidate that is not there")
        compare(screen.candidates.length, 0)
        screen.destroy()
    }

    function test_a_candidate_without_a_usable_address_is_a_failure() {
        // The address is the only unforgeable way to tell two candidates apart,
        // so a candidate with none is not a candidate the user can choose
        // between.
        var missing = makeScreen({
            "generate_identity_slate":
                '{"slate":"s1","count":1,"candidates":[{"index":0,"path":0,"publicKey":"aa"}]}'
        })
        missing.requestSlate()
        compare(missing.phase, "failed", "a candidate with no address cannot be chosen between")
        missing.destroy()

        var notAString = makeScreen({
            "generate_identity_slate":
                '{"slate":"s1","count":1,"candidates":[{"index":0,"path":0,'
                + '"address":{"hex":"aa"},"publicKey":"aa"}]}'
        })
        notAString.requestSlate()
        compare(notAString.phase, "failed", "nor one whose address is not a string")
        notAString.destroy()
    }

    function test_the_keep_guard_refuses_at_the_sentinel_even_when_a_row_carries_it() {
        // `keepSelected()` once had TWO guards, and the first was unprotected:
        // every fixture's candidates had non-negative indexes, so
        // `candidateAt(-1)` found nothing and the second caught the call. This
        // drives the refusal by making the sentinel addressable.
        //
        // The two have since collapsed into one — see `design.md`, "Selection
        // is a candidate's own index, and the sentinel is unaddressable", which
        // records why a second guard that can only be true when the first is is
        // not a guard.
        //
        // With the fix, such a slate never reaches the slate phase at all — so
        // the assertion is that no keep request is sent, which holds whether
        // the refusal comes from the guard or from the slate being refused
        // earlier. Both are the screen declining to keep something the user did
        // not choose.
        var screen = makeScreen({
            "generate_identity_slate": spec.slateWithNegativeIndex,
            "keep_identity": '{"kept":true,"address":"ff","publicKey":"gg","path":0,"encrypted":false}'
        })
        screen.requestSlate()
        compare(screen.selectedIndex, -1, "the fixture's own precondition")

        screen.keepSelected()

        compare(spec.countOf("keep_identity"), 0,
                "nothing is selected, so no keep may reach the bridge — not even "
                + "when a candidate carries the sentinel as its index")
        verify(screen.phase !== "kept",
               "and the reply above WOULD have produced the kept state had the "
               + "call been made")
        screen.destroy()
    }

    // Counts the rows that are VISIBLY marked as chosen, by walking the live
    // object tree for a shown Text reading SELECTED. `visible` is checked up
    // the chain, because an element is only on screen if every ancestor is.
    function visibleSelectedMarkers(item) {
        var items = spec.everyTextItemOn(item)
        var n = 0
        for (var i = 0; i < items.length; i++) {
            if (String(items[i].text) === "SELECTED" && spec.isShown(items[i]))
                n++
        }
        return n
    }

    function isShown(item) {
        var node = item
        while (node !== null && node !== undefined) {
            if (node.visible === false)
                return false
            node = node.parent
        }
        return true
    }

    function test_nothing_is_selected_when_a_set_arrives() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        compare(screen.selectedIndex, -1,
                "a pre-selected candidate plus one press is a permanent choice "
                + "the user never made")
        screen.destroy()
    }

    function test_selecting_one_candidate_deselects_any_other() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        screen.select(0)
        screen.select(1)

        compare(screen.selectedIndex, 1, "only the second selection stands")
        screen.destroy()
    }

    function test_selecting_reaches_the_module_not_at_all() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()
        var before = spec.calls.length

        screen.select(0)
        screen.select(1)
        screen.select(0)

        compare(spec.calls.length, before,
                "nothing is written until a candidate is kept, so selecting "
                + "must tell the module nothing")
        screen.destroy()
    }

    function test_keeping_is_refused_by_the_view_while_nothing_is_selected() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"ff","publicKey":"gg","path":0,"encrypted":false}'
        })
        screen.requestSlate()
        compare(screen.selectedIndex, -1)

        screen.keepSelected()

        compare(spec.countOf("keep_identity"), 0,
                "no keep request may reach the bridge while nothing is selected")
        verify(screen.phase !== "kept",
               "and the screen must not reach the kept state either — the reply "
               + "above WOULD have produced it had the call been made")
        screen.destroy()
    }

    function test_keeping_names_the_set_and_the_position() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"ff","publicKey":"gg","path":1,"encrypted":false}'
        })
        screen.requestSlate()
        screen.select(1)

        screen.keepSelected()

        compare(spec.countOf("keep_identity"), 1)
        // Parsed rather than string-matched, so field order in the request does
        // not decide whether this passes.
        var sent = JSON.parse(spec.calls[spec.calls.length - 1].request)
        compare(sent.slate, "feed01", "the request names the set the candidate was offered in")
        compare(sent.index, 1, "and the position it was selected at")
        compare(sent.stoa, "ab".repeat(32))
        screen.destroy()
    }

    function test_no_request_names_an_identity() {
        // The identity follows from the Stoa and the selection; a request
        // naming one would ask the module to act as somebody it is not.
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"ff","publicKey":"gg","path":0,"encrypted":true}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}'
        })
        screen.requestSlate()
        screen.select(0)
        screen.keepSelected()

        verify(spec.calls.length >= 2, "the fixture's own precondition")
        for (var i = 0; i < spec.calls.length; i++) {
            var request = JSON.parse(spec.calls[i].request)
            verify(request.address === undefined,
                   spec.calls[i].method + " must not name an address as the identity")
            verify(request.publicKey === undefined,
                   spec.calls[i].method + " must not name a public key")
            verify(request.path === undefined,
                   spec.calls[i].method + " must not name a derivation path")
            verify(request.identity === undefined,
                   spec.calls[i].method + " must not name an identity")
        }
        screen.destroy()
    }

    function test_each_call_reaches_the_bridge_under_its_own_method_name() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"ff","publicKey":"gg","path":0,"encrypted":true}'
        })
        screen.requestSlate()
        screen.select(0)
        screen.keepSelected()

        compare(spec.calls[0].method, "generate_identity_slate")
        compare(spec.calls[1].method, "keep_identity")
        verify(spec.calls[0].method !== spec.calls[1].method,
               "each call must reach the bridge naming a DIFFERENT method")
        screen.destroy()
    }

    // ---- the three keep outcomes ----------------------------------------
    //
    // THE trap this screen exists for. Core answers a refused keep with
    // `{"kept":false,"reason":…}` — a SUCCESS at the wire level, which
    // `Core.call()` correctly reports as `ok:true`. A screen branching on `ok`
    // alone reads it as a keep that worked.

    function test_a_refusal_is_neither_the_kept_state_nor_the_failed_state() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":false,"reason":"an identity already exists for this Stoa"}'
        })
        screen.requestSlate()
        screen.select(0)

        screen.keepSelected()

        verify(screen.phase !== "kept",
               "a kept:false reply arrives as ok:true and must NOT reach the kept state")
        verify(screen.phase !== "failed",
               "nor the failed state — the module answered the question it was asked")
        compare(screen.phase, "refused")
        compare(screen.refusal, "an identity already exists for this Stoa",
                "the reason is shown as the module wrote it")
        compare(screen.keptIdentity, null,
                "no identity may be reported on a reply that stored none")
        screen.destroy()
    }

    function test_a_refusal_leaves_the_candidates_on_screen_and_the_keep_repeatable() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":false,"reason":"the store is locked"}'
        })
        screen.requestSlate()
        screen.select(1)
        screen.keepSelected()

        compare(screen.candidates.length, 2, "the set that was on screen is still on screen")
        compare(screen.candidates[0].address, spec.addrA)
        compare(screen.selectedIndex, 1, "and the selection survives, so retrying is one press")

        screen.keepSelected()
        compare(spec.countOf("keep_identity"), 2, "the keep can be invoked again")
        screen.destroy()
    }

    function test_a_non_string_reason_does_not_render_as_object_Object() {
        // Regression, correctness review: `String(reply.value.reason)` on an
        // object yields the literal text `[object Object]`, which the screen
        // then showed the user in place of a reason. The spec requires a
        // refusal's reason be shown "as the module wrote it, since a module's
        // reasons are written to name a fix" — `[object Object]` names no fix
        // and is strictly worse than the screen's own fallback, which exists
        // for exactly the case where no usable reason arrived.
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":false,"reason":{"code":7}}'
        })
        screen.requestSlate()
        screen.select(0)

        screen.keepSelected()

        compare(screen.phase, "refused", "it is still a refusal")
        verify(screen.refusal.indexOf("[object Object]") < 0,
               "a non-string reason must not be stringified onto the screen, got: "
               + screen.refusal)
        verify(screen.refusal.length > 0,
               "and the screen must still say something rather than showing a blank")
        screen.destroy()
    }

    function test_a_non_string_slate_identifier_is_not_stringified_into_a_request() {
        // The same coercion at the other `String()` call site. A slate
        // identifier that is not a string is a reply the screen cannot read,
        // not one to stringify and send back.
        var screen = makeScreen({
            "generate_identity_slate":
                '{"slate":{"nonce":"aa"},"count":1,"candidates":[{"index":0,"path":0,'
                + '"address":"' + "11".repeat(32) + '","publicKey":"aa"}]}'
        })
        screen.requestSlate()

        compare(screen.phase, "failed",
                "a set identifier the screen cannot read is a reply it cannot read")
        verify(screen.slateId.indexOf("[object Object]") < 0)
        screen.destroy()
    }

    function test_a_new_slate_clears_a_previously_kept_identity() {
        // Regression, correctness review: `requestSlate()`'s success path
        // cleared `refusal` and `failure` but not `keptIdentity`, while
        // `enterFailed()` cleared it — so the two exits from a state disagreed
        // about what they clean up, and the next reader of `keptIdentity`
        // inherited a stale value for free.
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"' + "99".repeat(32)
                           + '","publicKey":"pk","path":0,"encrypted":true}'
        })
        screen.requestSlate()
        screen.select(0)
        screen.keepSelected()
        compare(screen.phase, "kept", "the fixture's own precondition")
        verify(screen.keptIdentity !== null)

        screen.requestSlate()

        compare(screen.phase, "slate")
        compare(screen.keptIdentity, null,
                "a screen back on a slate holds no kept identity — the state "
                + "machine the spec asks to be single-valued must not carry a "
                + "second, stale answer alongside it")
        screen.destroy()
    }

    function test_a_kept_reply_is_the_kept_state_and_shows_the_replys_address() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"' + "99".repeat(32)
                           + '","publicKey":"pk","path":4,"encrypted":true}'
        })
        screen.requestSlate()
        screen.select(0)

        screen.keepSelected()

        compare(screen.phase, "kept")
        compare(screen.keptIdentity.address, "99".repeat(32))
        screen.destroy()
    }

    function test_the_identity_shown_is_the_replys_not_the_rows() {
        // The reply's address differs from the address of the row that was
        // selected. The screen must show the REPLY's: it is the one that is
        // true about the store.
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"' + "77".repeat(32)
                           + '","publicKey":"pk","path":0,"encrypted":false}'
        })
        screen.requestSlate()
        screen.select(0)
        compare(screen.candidates[0].address, spec.addrA, "the fixture's own precondition")

        screen.keepSelected()

        compare(screen.keptIdentity.address, "77".repeat(32))
        verify(screen.keptIdentity.address !== spec.addrA,
               "the row that was sent must not be what is shown")
        screen.destroy()
    }

    function test_a_failed_keep_is_the_failed_state_and_not_the_kept_one() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"error":"the keystore could not be written"}'
        })
        screen.requestSlate()
        screen.select(0)

        screen.keepSelected()

        compare(screen.phase, "failed")
        verify(screen.phase !== "kept")
        compare(screen.keptIdentity, null)
        compare(screen.failure, "the keystore could not be written")
        screen.destroy()
    }

    function test_a_kept_reply_with_no_address_is_a_failure_not_an_empty_identity() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"publicKey":"pk","path":0,"encrypted":true}'
        })
        screen.requestSlate()
        screen.select(0)

        screen.keepSelected()

        compare(screen.phase, "failed",
                "a kept identity with an empty address claims a success the reply "
                + "did not describe")
        compare(screen.keptIdentity, null)
        verify(screen.failure.length > 0, "the failure must name itself")
        screen.destroy()
    }

    function test_the_kept_signal_fires_only_on_a_reply_that_kept_something() {
        var kept = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"ab","publicKey":"pk","path":0,"encrypted":false}'
        })
        var fired = 0
        kept.identityKept.connect(function () { fired++ })
        kept.requestSlate()
        kept.select(0)
        kept.keepSelected()
        compare(fired, 1)
        kept.destroy()

        var refused = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":false,"reason":"no"}'
        })
        var refusedFired = 0
        refused.identityKept.connect(function () { refusedFired++ })
        refused.requestSlate()
        refused.select(0)
        refused.keepSelected()
        compare(refusedFired, 0,
                "a refusal must not tell the rest of the view an identity exists")
        refused.destroy()
    }

    // ---- the encryption and recovery reports -----------------------------

    function test_the_two_encryption_replies_produce_different_text() {
        // Not "the screen says something" but "the screen says something
        // DIFFERENT", which is the property that makes the state readable
        // rather than implied.
        var encrypted = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"ab","publicKey":"pk","path":0,"encrypted":true}'
        })
        encrypted.requestSlate()
        encrypted.select(0)
        encrypted.keepSelected()
        compare(encrypted.keptIdentity.encrypted, true)

        var plain = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"ab","publicKey":"pk","path":0,"encrypted":false}'
        })
        plain.requestSlate()
        plain.select(0)
        plain.keepSelected()
        compare(plain.keptIdentity.encrypted, false)

        verify(encrypted.keptIdentity.encrypted !== plain.keptIdentity.encrypted,
               "the two replies must not collapse to one state")
        encrypted.destroy()
        plain.destroy()
    }

    function test_an_omitted_encryption_field_is_not_read_as_a_negative_answer() {
        // An absent `false` and a reported `false` mean different things: one
        // is "the module did not say", the other is "your key is in the clear".
        // A view normalising with `=== true` would report the second.
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":true,"address":"ab","publicKey":"pk","path":0}'
        })
        screen.requestSlate()
        screen.select(0)
        screen.keepSelected()

        compare(screen.phase, "kept")
        compare(screen.keptIdentity.encrypted, undefined,
                "an omitted field must stay distinguishable from a reported false")
        verify(screen.keptIdentity.encrypted !== false,
               "and must NOT have been coerced to false")
        screen.destroy()
    }

    function test_an_omitted_recovery_field_produces_no_claim() {
        var screen = makeScreen({})
        compare(screen.recoveryNeedsTheRecord, undefined)
        // Set as the module would report it, and then as it would omit it.
        screen.recoveryNeedsTheRecord = true
        compare(screen.recoveryNeedsTheRecord, true)
        screen.recoveryNeedsTheRecord = undefined
        verify(screen.recoveryNeedsTheRecord !== false,
               "an absent recovery field must not read as a reported false")
        screen.destroy()
    }

    // ---- what the screen must never say ---------------------------------

    function test_no_copy_on_this_screen_uses_the_word_username() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        var texts = spec.everyTextOn(screen)
        verify(texts.length > 5, "the sweep must actually be finding text, got "
                                 + texts.length + " strings")
        for (var i = 0; i < texts.length; i++) {
            verify(texts[i].toLowerCase().indexOf("username") < 0,
                   "the word username must never appear, found in: " + texts[i])
        }
        screen.destroy()
    }

    function test_no_copy_claims_the_identity_cannot_be_linked_elsewhere() {
        // The bundle's body copy ends "the key is yours in this Stoa only — it
        // cannot be linked to you anywhere else". One key signs in every Stoa
        // in this release, so that is false, and an interface telling a user
        // they have a privacy property they do not have is the one failure here
        // that could actually harm someone.
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        var texts = spec.everyTextOn(screen)
        var forbidden = ["cannot be linked", "in this Stoa only", "anywhere else",
                         "unlinkable", "not be linked"]
        for (var i = 0; i < texts.length; i++) {
            for (var j = 0; j < forbidden.length; j++) {
                verify(texts[i].indexOf(forbidden[j]) < 0,
                       "no copy may claim cross-Stoa unlinkability; found \""
                       + forbidden[j] + "\" in: " + texts[i])
            }
        }
        screen.destroy()
    }

    function test_the_true_half_of_the_claim_is_still_made() {
        // Saying less is available; saying nothing is not. The screen must
        // still state that a key is being picked and that the name follows from
        // it and cannot be changed.
        var screen = makeScreen({})
        var joined = spec.everyTextOn(screen).join(" ")

        verify(joined.indexOf("picking a key") >= 0,
               "the screen must say a key is being picked")
        verify(joined.indexOf("computed from it") >= 0,
               "and that the name is computed from it")
        verify(joined.indexOf("cannot be changed") >= 0,
               "and that it cannot be changed afterwards")
        screen.destroy()
    }

    function test_the_uniqueness_note_states_the_obligation_without_a_word_count() {
        // **No count, in the copy or in this assertion.** The count has now
        // moved three times — three words, then four (merged), now three again
        // on a different basis (adjective + noun + "of" + place) — and each
        // move made the shipped copy wrong and the pinning test an obstacle to
        // correcting it. A pin on a number fails on reword rather than on
        // misinformation, which is the wrong failure.
        //
        // The sentence's obligation does not need a number. What it must say is
        // that names are not unique, are not identifiers, that someone else may
        // hold the same name, and that the ADDRESS is what distinguishes two
        // participants — and that survives every future change to how many
        // words a name has.
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()
        var joined = spec.everyTextOn(screen).join(" ")

        verify(joined.indexOf("not unique") >= 0,
               "the note must say names are not unique")
        verify(joined.indexOf("not identifiers") >= 0,
               "and that they are not identifiers")
        verify(joined.indexOf("address") >= 0,
               "and that the address is what tells participants apart")

        // And no count ships, whatever the count currently is. Written as a
        // sweep over the spellings rather than a pin on one, so this fails on
        // the reintroduction of ANY number rather than on the reword of one.
        var counts = ["two words", "three words", "four words", "five words",
                      "2 words", "3 words", "4 words", "5 words"]
        for (var i = 0; i < counts.length; i++) {
            verify(joined.indexOf(counts[i]) < 0,
                   "the copy must state no word count — it has changed three "
                   + "times and a screen carrying a number goes stale on the "
                   + "next move; found \"" + counts[i] + "\"")
        }
        screen.destroy()
    }

    function test_the_uniqueness_obligation_survives_without_the_apparatus_column() {
        // The spec requires this screen to state that names are not unique and
        // not identifiers. That obligation must not rest on the apparatus
        // column, which is annotation rather than interface and which a sibling
        // piece removes — a spec'd requirement deleted as a side effect of
        // dropping decoration is the failure this pins.
        //
        // So: require the statement to appear on TWO separate elements. One is
        // the margin note; the second is the body copy, and it is the one that
        // survives the column's removal. With the margin note alone this finds
        // one and fails.
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        var texts = spec.everyTextOn(screen)
        var carriers = 0
        for (var i = 0; i < texts.length; i++) {
            if (texts[i].indexOf("not unique") >= 0
                && texts[i].indexOf("not identifiers") >= 0)
                carriers++
        }

        compare(carriers, 2,
                "the obligation must be stated in the body as well as the "
                + "margin, so dropping the apparatus column cannot delete a "
                + "spec'd requirement")
        screen.destroy()
    }

    function test_the_permanence_and_refresh_notes_are_on_screen_with_the_candidates() {
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()
        var joined = spec.everyTextOn(screen).join(" ")

        verify(joined.indexOf("There is no settings screen where this can be changed later") >= 0,
               "the permanence warning must be on screen with the candidates")
        verify(joined.indexOf("Refresh as often as you like. Nothing is published until you keep one.") >= 0,
               "and the statement that refreshing costs nothing")
        screen.destroy()
    }

    function test_every_text_element_on_the_screen_is_explicitly_plain_text() {
        // QML's default is AutoText, which SNIFFS its input and switches to
        // rich text when a string looks like markup. An element left on the
        // default is one binding change away from rendering markup.
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        var items = spec.everyTextItemOn(screen)
        verify(items.length > 5, "the sweep must actually be finding Text items, got "
                                 + items.length)
        for (var i = 0; i < items.length; i++) {
            compare(items[i].textFormat, Text.PlainText,
                    "every Text must be PlainText, not AutoText: \"" + items[i].text + "\"")
        }
        screen.destroy()
    }

    function test_a_refusal_reason_containing_markup_is_shown_as_its_characters() {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": '{"kept":false,"reason":"<b>locked</b> by /x/keys"}'
        })
        screen.requestSlate()
        screen.select(0)
        screen.keepSelected()

        compare(screen.refusal, "<b>locked</b> by /x/keys",
                "the reason is held as its characters, not reworded")

        // And the element that renders it does not interpret them.
        var items = spec.everyTextItemOn(screen)
        var found = false
        for (var i = 0; i < items.length; i++) {
            if (items[i].text === "<b>locked</b> by /x/keys") {
                found = true
                compare(items[i].textFormat, Text.PlainText,
                        "the markup must be rendered as characters, not as formatting")
            }
        }
        verify(found, "the reason must actually be rendered somewhere on the screen")
        screen.destroy()
    }

    function test_no_row_presents_a_derivation_path_or_an_index_as_a_name() {
        // Core carries no generated name and the view cannot compute one. A row
        // must leave the name UNSHOWN rather than substituting the path, the
        // index, a position number or a truncated address — each would be read
        // as the thing being chosen, and none of them is.
        var screen = makeScreen({
            "generate_identity_slate":
                '{"slate":"s1","count":1,"candidates":[{"index":0,"path":7,"address":"'
                + "44".repeat(32) + '","publicKey":"pk"}]}'
        })
        screen.requestSlate()

        var texts = spec.everyTextOn(screen)
        for (var i = 0; i < texts.length; i++) {
            var t = texts[i]
            // The full address is legitimately on screen; nothing else derived
            // from the candidate may be.
            if (t === "44".repeat(32))
                continue
            verify(t !== "7" && t !== "0" && t !== "1" && t !== "#1",
                   "a path or an index must not stand where a name would: \"" + t + "\"")
            verify(t !== "pk", "nor the public key")
        }
        screen.destroy()
    }

    function test_at_most_one_of_kept_refused_and_failed_is_ever_the_phase() {
        // The states are mutually exclusive BY CONSTRUCTION: one string, not
        // three booleans. Drive the screen through each outcome and assert the
        // phase is exactly one recognised value each time.
        var outcomes = [
            { keep: '{"kept":true,"address":"ab","publicKey":"pk","path":0,"encrypted":true}',
              expected: "kept" },
            { keep: '{"kept":false,"reason":"nope"}', expected: "refused" },
            { keep: '{"error":"boom"}', expected: "failed" }
        ]
        var seen = []
        for (var i = 0; i < outcomes.length; i++) {
            var screen = makeScreen({
                "generate_identity_slate": spec.twoCandidateSlate,
                "keep_identity": outcomes[i].keep
            })
            screen.requestSlate()
            screen.select(0)
            screen.keepSelected()
            compare(screen.phase, outcomes[i].expected)
            seen.push(screen.phase)
            screen.destroy()
        }
        // And the three outcomes did not collapse into one another.
        verify(seen[0] !== seen[1] && seen[1] !== seen[2] && seen[0] !== seen[2],
               "three outcomes must reach three distinct phases, got " + seen.join(","))
    }

    // ---- sweeps ----------------------------------------------------------
    //
    // These walk the live object tree rather than a hand-maintained list of
    // elements, so a Text added to the screen later is covered without this
    // file being edited. A hand-kept list goes stale silently; a walk does not.

    function everyTextItemOn(item) {
        var found = []
        spec.collectText(item, found)
        return found
    }

    function everyTextOn(item) {
        var items = spec.everyTextItemOn(item)
        var texts = []
        for (var i = 0; i < items.length; i++) {
            if (items[i].text !== undefined && String(items[i].text) !== "")
                texts.push(String(items[i].text))
        }
        return texts
    }

    function collectText(item, out) {
        if (item === null || item === undefined)
            return
        // A Text (and anything deriving from one, e.g. AddressLabel) has both a
        // `text` and a `textFormat`. Duck-typing rather than a type check,
        // because `instanceof Text` is not available to QML's JS.
        if (item.textFormat !== undefined && item.text !== undefined)
            out.push(item)
        var kids = item.children
        if (kids === undefined)
            return
        for (var i = 0; i < kids.length; i++)
            spec.collectText(kids[i], out)
    }
}
