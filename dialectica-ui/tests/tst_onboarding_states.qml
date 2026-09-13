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

    function test_the_sentinel_addresses_no_candidate_the_screen_will_hold() {
        // The PREMISE the single guard rests on, pinned in its own right rather
        // than inherited from the two tests that happen to exercise it.
        //
        // `keepSelected()` has one guard — `candidateAt(selectedIndex) === null`
        // — and that is sufficient only while the sentinel addresses nothing. If
        // `isCandidate()` ever admitted a negative index again, the sentinel
        // would name a real candidate, `candidateAt()` would return it, and the
        // one guard would wave a keep through for a candidate nobody selected.
        // The two tests above would still pass: the first asserts a slate is
        // refused, the second asserts a keep is not sent — both hold as long as
        // the refusal happens SOMEWHERE, which is why neither pins the reason.
        //
        // So drive the invariant itself, over the sentinel's value and over
        // every negative spelling of a position. `candidateAt` is asserted to
        // find nothing at the sentinel BOTH on a slate that was accepted and on
        // one whose negative candidate was refused, so the assertion cannot pass
        // merely because the list is empty.
        var good = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        good.requestSlate()
        compare(good.phase, "slate", "the fixture's own precondition")
        compare(good.candidates.length, 2,
                "and the list is NOT empty, so finding nothing below is a fact "
                + "about the sentinel rather than about an empty screen")
        compare(good.candidateAt(good.nothingSelected), null,
                "the sentinel must address no candidate of an accepted slate")
        compare(good.candidateAt(0), good.candidates[0],
                "while a real position still finds its own candidate — without "
                + "this, a candidateAt() that always returned null would pass")
        good.destroy()

        // And no negative index survives the boundary at all, whatever its
        // spelling. `-0` is excluded deliberately: it is `0` in JavaScript and a
        // legitimate first position, so listing it would be a case that must
        // NOT be refused.
        var negatives = [-1, -2, -7, -1.5]
        for (var i = 0; i < negatives.length; i++) {
            var hostile = makeScreen({
                "generate_identity_slate":
                    '{"slate":"s1","count":1,"candidates":[{"index":' + negatives[i]
                    + ',"path":0,"address":"' + "11".repeat(32) + '","publicKey":"aa"}]}'
            })
            hostile.requestSlate()
            compare(hostile.phase, "failed",
                    "a candidate carrying index " + negatives[i] + " must be refused "
                    + "at the boundary; the one keep guard is only sufficient while "
                    + "no candidate can carry the sentinel")
            compare(hostile.candidateAt(hostile.nothingSelected), null,
                    "and the sentinel addresses nothing afterwards either")
            hostile.destroy()
        }
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

    function test_the_slate_request_carries_the_stoa_it_was_made_for() {
        // Spec/test review finding: only the KEEP request's `stoa` was
        // asserted. `Core.qml:146` `{ stoa: stoa }` → `{}` passed 102/102 — a
        // slate generated for no Stoa at all, with the whole suite green. The
        // scenario "A request carries the fields its method reads" names the
        // slate call and the identity-report call explicitly.
        //
        // Asserted against the address THIS TEST supplied to the screen, not
        // against whatever the request happened to contain.
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        compare(spec.countOf("generate_identity_slate"), 1, "the fixture's own precondition")
        var sent = JSON.parse(spec.calls[0].request)
        compare(sent.stoa, "ab".repeat(32),
                "the slate request must carry the Stoa the screen was given — a "
                + "slate for the wrong Stoa, or for none, is what this scenario "
                + "exists to catch")
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

    // Drives a screen to the kept phase and returns the text that is ACTUALLY
    // SHOWN there, so the encryption and recovery assertions below are about
    // what a user reads rather than about the field the fixture supplied.
    //
    // `recovery` is the value `Main` would have set from the identity report;
    // it is assigned unconditionally, so passing `undefined` is the module
    // having omitted the field rather than this helper skipping a step.
    function keptScreenShowing(keepReply, recovery) {
        var screen = makeScreen({
            "generate_identity_slate": spec.twoCandidateSlate,
            "keep_identity": keepReply
        })
        screen.recoveryNeedsTheRecord = recovery
        screen.requestSlate()
        screen.select(0)
        screen.keepSelected()
        compare(screen.phase, "kept", "the fixture's own precondition: the keep reported a keep")
        return screen
    }

    function test_the_two_encryption_replies_produce_different_visible_text() {
        // Review finding (readability): the previous version of this test
        // compared `encrypted.keptIdentity.encrypted` to
        // `plain.keptIdentity.encrypted` — two reads of the value THE FIXTURE
        // SUPPLIED. It never looked at the Text its name promises, so
        // collapsing the render site's three-way conditional to the encrypted
        // arm alone left 99 of 99 tests green while the screen told a user
        // whose master key sits in the clear that it is encrypted. The spec
        // singles this out: "protection that reads as strong while being absent
        // is worse than visible plaintext".
        //
        // So this reads the RENDERED, VISIBLE text and pins each reply to its
        // own hardcoded phrase. Asserting only that the two differ would not be
        // enough — three refusals on a sibling piece were asserted to be three
        // DIFFERENT strings and stayed green when one was reworded to actively
        // misinform. Different is necessary; correct is the requirement.
        var encrypted = spec.keptScreenShowing(
            '{"kept":true,"address":"ab","publicKey":"pk","path":0,"encrypted":true}')
        var encryptedText = spec.visibleTextMatching(encrypted, "master key on this machine")
        var plain = spec.keptScreenShowing(
            '{"kept":true,"address":"ab","publicKey":"pk","path":0,"encrypted":false}')
        var plainText = spec.visibleTextMatching(plain, "master key on this machine")

        // Each reply's own claim, hardcoded here rather than read back off the
        // screen. A reply reporting encryption must not produce the in-the-clear
        // sentence, and — the safety-relevant direction — a reply reporting NO
        // encryption must never produce the reassuring one.
        verify(encryptedText.indexOf("stored encrypted") >= 0,
               "a reply reporting an encrypted key must say so; the screen shows: \""
               + encryptedText + "\"")
        verify(encryptedText.indexOf("in the clear") < 0,
               "and must not also say the key is in the clear: \"" + encryptedText + "\"")

        verify(plainText.indexOf("in the clear") >= 0,
               "a reply reporting an UNENCRYPTED key must say so plainly; the screen "
               + "shows: \"" + plainText + "\"")
        verify(plainText.indexOf("stored encrypted") < 0,
               "and must NEVER tell a user whose key is in the clear that it is "
               + "encrypted — that is the one false reassurance this screen could "
               + "make; the screen shows: \"" + plainText + "\"")

        // No lock, no shield, no "secure" — the spec forbids protection that
        // reads as strong while being absent.
        var plainWords = spec.visibleTextsOn(plain).join(" ").toLowerCase()
        verify(plainWords.indexOf("secure") < 0,
               "an unencrypted key must not be described as secure")

        verify(encryptedText !== plainText,
               "and the two replies must not collapse to one state")
        encrypted.destroy()
        plain.destroy()
    }

    function test_an_omitted_encryption_field_shows_no_claim_on_screen() {
        // The third case, and the one a two-armed conditional silently swallows:
        // a reply that OMITTED the field must produce NO sentence at all, not
        // the negative one. An absent `false` and a reported `false` mean
        // different things.
        var screen = spec.keptScreenShowing(
            '{"kept":true,"address":"ab","publicKey":"pk","path":0}')

        var shown = spec.visibleTextsOn(screen)
        // Both bounds. The corpus must be non-trivial (or "nothing is shown"
        // would satisfy the absence half vacuously) and must reach the kept
        // card, proving the walk got as far as the sentence it says is absent.
        verify(shown.length > 5, "the visible sweep must be finding text, got " + shown.length)
        verify(shown.join(" ").indexOf("This is who you are here now.") >= 0,
               "and must reach the kept card, or its silence proves nothing")

        for (var i = 0; i < shown.length; i++) {
            verify(shown[i].indexOf("master key on this machine") < 0,
                   "a reply that said nothing about protection must produce no claim "
                   + "about it, but the screen shows: \"" + shown[i] + "\"")
        }
        screen.destroy()
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

    function test_the_backup_gap_is_stated_only_when_the_module_reported_it() {
        // Review finding (readability): the previous version of this test set
        // `recoveryNeedsTheRecord` and read it back, which is a property
        // round-trip and not a claim about the screen. Replacing the render
        // site's `visible: screen.recoveryNeedsTheRecord === true` with
        // `visible: true` left 99 of 99 tests green while the backup-gap
        // sentence was shown to every kept user, including one whose module
        // never reported the field. The spec: "A reply omitting either field
        // SHALL produce no claim about it."
        //
        // Table rather than three near-identical functions, and each row states
        // what must be SHOWN — the sentence is a claim only when a user can
        // read it.
        var cases = [
            { reported: true,      expect: true,
              why: "the module said recovery needs more than the master key, so the "
                   + "screen must say the record lives on this device" },
            { reported: false,     expect: false,
              why: "the module said it does not, so no gap may be claimed" },
            { reported: undefined, expect: false,
              why: "the module said nothing, so the screen may claim nothing" }
        ]
        var gap = "recorded only on this machine"
        for (var i = 0; i < cases.length; i++) {
            var screen = spec.keptScreenShowing(
                '{"kept":true,"address":"ab","publicKey":"pk","path":0,"encrypted":true}',
                cases[i].reported)

            var shown = spec.visibleTextsOn(screen)
            // Both bounds again: prove the walk reaches the kept card before
            // trusting what it says is missing from it.
            verify(shown.join(" ").indexOf("This is who you are here now.") >= 0,
                   "the visible sweep must reach the kept card")

            var stated = shown.join(" ").indexOf(gap) >= 0
            compare(stated, cases[i].expect, cases[i].why)

            if (cases[i].expect) {
                // And where the gap IS stated, nothing beside it may present a
                // saved key as sufficient.
                var joined = spec.visibleTextsOn(screen).join(" ")
                verify(joined.indexOf("not enough to get back in") >= 0,
                       "the gap must say the master key alone is not enough")
            }
            screen.destroy()
        }
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

    // EVERY sentence this screen is allowed to say, as an exact set.
    //
    // Spec/test review finding: the cross-Stoa unlinkability guard was a
    // blocklist of five phrasings, and the reviewer appended *"This key stays
    // private to this Stoa and no observer can connect it to your other
    // Stoas."* — the false privacy claim the spec calls "the one failure here
    // that could actually harm someone" — and **49 of 49 tests passed**. It
    // matches no needle. A blocklist fails on an innocent reword and passes on
    // a fluent lie, which is the wrong way round for the property that could
    // hurt a user.
    //
    // So this is an allowlist, the same instrument that caught "Key H" in
    // `test_a_row_shows_its_address_and_its_mark_and_nothing_else`: the screen's
    // copy is entirely AUTHORED — no string here is computed — so the set of
    // sentences it can show is finite and writable down. Any new sentence,
    // anywhere, in any phase, fails until someone adds it here deliberately.
    // That is the point: adding a privacy claim then requires typing it into a
    // list headed by the reason it must not be typed.
    //
    // Module-supplied strings (refusal reasons, failure messages) and
    // candidate-derived strings (addresses) are excluded by the caller, because
    // those are not authored here and are pinned by their own tests.
    readonly property var authoredCopy: [
        // heading, shown in every phase
        "Choose the identity you will keep here.",
        "You are picking a key. Its name is computed from it, so the name cannot be changed afterwards.",
        // intro
        "You have no identity here yet. Nothing has been generated and nothing has been stored.",
        "Show me some keys",
        // slate
        "SELECTED",
        "Show me five more",
        "Refresh as often as you like. Nothing is published until you keep one.",
        "Keep this identity",
        "There is no settings screen where this can be changed later, because the name is only the key written out. Choosing again means being someone else here.",
        "Names are not unique and are not identifiers. Someone else in this Stoa may hold the same name. Your address is what tells you apart, so it is printed beside your name everywhere.",
        // refused
        "That identity was not kept.",
        "Nothing was stored. The keys above are still on offer.",
        // kept
        "This is who you are here now.",
        "The master key on this machine is stored encrypted.",
        "The master key on this machine is stored unencrypted, in the clear. Anyone who can read the file can use it.",
        "Which key you chose is recorded only on this machine. A copy of the master key by itself is not enough to get back in — it can derive this identity, but not tell you which one was yours.",
        // failed
        "No identity could be offered, so nothing can be chosen yet.",
        "Nothing was stored and nothing was lost. This is a failure to read or write on this machine, not a choice that went wrong.",
        "Try again",
        // apparatus
        "APPARATUS",
        "ON PERMANENCE",
        "ON UNIQUENESS",
        "ON THE MARK",
        "The hatched shape is drawn from the same key: two inks for the weave, a third for the outline, all three chosen by the key. A curved contour always means a person; an angular one always means a Stoa."
    ]

    function test_the_screen_says_only_what_it_is_allowed_to_say() {
        // Drives every phase, because the walk reaches a phase's copy whether or
        // not it is visible but a phase's copy only EXISTS once that phase has
        // been entered — the refused and kept cards are built from replies.
        var phases = [
            { keep: undefined },
            { keep: '{"kept":false,"reason":"REASON-FIXTURE"}' },
            { keep: '{"kept":true,"address":"KEPT-ADDRESS-FIXTURE","publicKey":"pk",'
                    + '"path":0,"encrypted":true}' }
        ]
        for (var p = 0; p < phases.length; p++) {
            var replies = { "generate_identity_slate": spec.twoCandidateSlate }
            if (phases[p].keep !== undefined)
                replies["keep_identity"] = phases[p].keep
            var screen = makeScreen(replies)
            screen.requestSlate()
            if (phases[p].keep !== undefined) {
                screen.select(0)
                screen.keepSelected()
            }

            var texts = spec.everyTextOn(screen)
            // Both bounds. A neutered sweep satisfies an allowlist vacuously,
            // which is the exact failure the reviewer measured on the blocklist
            // this replaces: `everyTextOn()` → `return []` left it green.
            verify(texts.length > 15,
                   "the sweep must be finding the screen's copy, got " + texts.length)

            for (var i = 0; i < texts.length; i++) {
                var t = texts[i]
                // Not authored HERE, and pinned elsewhere: every string the
                // module supplied. Each is a distinctive fixture value rather
                // than a pattern, so this skips exactly what this test handed
                // the screen and nothing else — a wildcard here would be a hole
                // an unauthorised sentence could be written through.
                if (t === spec.addrA || t === spec.addrB)
                    continue                                   // candidate addresses
                if (t === "REASON-FIXTURE")
                    continue                                   // the refusal reason
                if (t === "KEPT-ADDRESS-FIXTURE")
                    continue                                   // the kept reply's address
                verify(spec.authoredCopy.indexOf(t) >= 0,
                       "this screen said something no test authorised:\n    \"" + t
                       + "\"\nEvery sentence the screen shows must be in "
                       + "`authoredCopy` above. If this is a deliberate copy "
                       + "change, add it there — and if it is a claim about "
                       + "privacy, linkability or what an observer can see, read "
                       + "the ninth requirement first: one key signs in every "
                       + "Stoa in this release, so the screen must not say or "
                       + "imply otherwise.")
            }
            screen.destroy()
        }
    }

    function test_no_copy_claims_the_identity_cannot_be_linked_elsewhere() {
        // The allowlist above is what actually catches an unanticipated
        // phrasing. This stays as the NAMED guard for the specific claim,
        // because a reader looking for "where is cross-Stoa unlinkability
        // pinned?" should find a test called that — and because the two fail
        // differently: the allowlist says "nobody authorised this sentence",
        // this says "that sentence is the forbidden claim".
        //
        // The corpus guard is the fix for the second half of the review
        // finding: this was the only `everyTextOn` caller with no length bound,
        // so neutering the helper left it vacuously green.
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        var texts = spec.everyTextOn(screen)
        verify(texts.length > 15,
               "the sweep must be finding the screen's copy, got " + texts.length)

        var joined = texts.join(" ").toLowerCase()
        var forbidden = ["cannot be linked", "in this stoa only", "anywhere else",
                         "unlinkable", "not be linked", "stays private to this stoa",
                         "connect it to your other", "no observer", "only this stoa",
                         "cannot be connected", "not be connected", "cannot be traced",
                         "cannot be tied"]
        for (var j = 0; j < forbidden.length; j++) {
            verify(joined.indexOf(forbidden[j]) < 0,
                   "no copy may claim cross-Stoa unlinkability; found \""
                   + forbidden[j] + "\" on screen")
        }
        screen.destroy()
    }

    function test_the_true_half_of_the_claim_is_still_made() {
        // Saying less is available; saying nothing is not. The screen must
        // still state that a key is being picked and that the name follows from
        // it and cannot be changed.
        var screen = makeScreen({})
        var texts = spec.everyTextOn(screen)
        var joined = texts.join(" ")

        // Spec/test review finding: the scenario "The opening state says an
        // identity is being chosen" pins a LITERAL heading, and no test
        // contained it — replacing it with "Set up your account" passed 49/49.
        // That is account-setup framing, which PLAN §5.2.1 says generates a
        // support question that cannot be answered ("how do I change my
        // username?"), so the wording is the requirement rather than decoration.
        // Pinned as an exact element, not a substring of the joined copy.
        verify(texts.indexOf("Choose the identity you will keep here.") >= 0,
               "the spec states this heading exactly; the screen must show it "
               + "verbatim rather than a paraphrase, because account-setup "
               + "framing teaches the one mental model this flow cannot honour")

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

        // And no count ships, whatever the count currently is. The SHAPE here —
        // assert no number at all rather than pin the current one — is right and
        // was endorsed on review: a pin on a number fails on reword rather than
        // on misinformation.
        //
        // The needle list under it was not. Spec/test review measured it: the
        // eight literals missed `"three-word"` — hyphenated and attributive,
        // which is the most natural way a count comes back ("Your three-word
        // name is not unique…") — and 49 of 49 passed with a word count on
        // screen. `"a pair of words"`, `"3-word"` and any capitalised form were
        // missed too, since unlike the `username` sweep this one did not
        // lowercase.
        //
        // Two regexes over the lowercased copy replace the list, so this fails
        // on a spelling nobody enumerated rather than on one somebody did.
        var lower = joined.toLowerCase()
        var numberWord = "(one|two|three|four|five|six|seven|eight|nine|ten|[0-9]+)"
        var patterns = [
            // "three words", "three-word", "3 word", "3-word", and the plural
            // and attributive forms of each.
            new RegExp(numberWord + "[ -]words?\\b"),
            // "a pair of words", "a trio of words", "a couple of words"
            new RegExp("\\b(a pair|a trio|a couple|a set) of words\\b"),
            // "words: three", "name is three", i.e. the count stated after the noun
            new RegExp("\\b(name|names|words) (is|are|of) " + numberWord + "\\b")
        ]
        for (var i = 0; i < patterns.length; i++) {
            var hit = lower.match(patterns[i])
            verify(hit === null,
                   "the copy must state no word count — it has changed three "
                   + "times and a screen carrying a number goes stale on the "
                   + "next move; found \"" + (hit === null ? "" : hit[0]) + "\"")
        }

        // The regexes above are themselves assertable, and a regex that matched
        // nothing would make the sweep silently vacuous — the same defect as a
        // neutered corpus, one level down. So prove each pattern fires on the
        // exact reintroduction the reviewer measured, and on the literal forms
        // the old list covered.
        var mustBeCaught = [
            "your three-word name is not unique",
            "the same three words",
            "the same 3 words",
            "a pair of words",
            "the name is three"
        ]
        for (var m = 0; m < mustBeCaught.length; m++) {
            var caught = false
            for (var q = 0; q < patterns.length; q++) {
                if (mustBeCaught[m].match(patterns[q]) !== null)
                    caught = true
            }
            verify(caught,
                   "the count sweep must catch \"" + mustBeCaught[m] + "\" — a "
                   + "pattern that matches nothing makes this test vacuous, "
                   + "which is the defect it was written to fix")
        }
        screen.destroy()
    }

    function test_the_uniqueness_obligation_survives_without_the_apparatus_column() {
        // Design review finding: this asserted `compare(carriers, 2)` — EXACTLY
        // two — so it **failed in exactly the scenario its name promises to
        // survive**. Removing the `ON UNIQUENESS` margin note, which is what
        // `piece/drop-apparatus` (#70) produces, gave `Actual: 1 Expected: 2`.
        //
        // That inverted the rule it was cited as enforcing. `design.md` states
        // it as *"apparatus may repeat an obligation, never carry it alone"* —
        // repetition is PERMITTED, not required, and `ON THE MARK` is
        // legitimately margin-only. An exact count turns the rule into a
        // requirement to keep the column, and the cheapest green for a
        // `drop-apparatus` author is to edit the `2`: a count-pin on decoration
        // that is being deleted, which is the same failure mode this change's
        // own word-count decision argues against, one file away.
        //
        // What the rule actually wants is that the obligation has a carrier
        // OUTSIDE the apparatus column. That passes today, passes after the
        // column goes, and fails only if the body copy is dropped — which is
        // the defect. So the assertion is on the property, not the count.
        var screen = makeScreen({ "generate_identity_slate": spec.twoCandidateSlate })
        screen.requestSlate()

        // Both counts come from the SAME walk (`everyTextOn`), so the
        // subtraction below is coherent. Counting the column with a different
        // walk than the whole screen lets the two desynchronise — measured:
        // neutering `everyTextOn` alone produced "Found 0 carrier(s), 1 of them
        // in apparatus", a negative remainder that happened to fail for the
        // right reason by luck rather than by arithmetic.
        var apparatusTexts = []
        for (var a = 0; a < screen.apparatus.length; a++)
            apparatusTexts = apparatusTexts.concat(spec.everyTextOn(screen.apparatus[a]))

        var carriesIt = function (s) {
            return s.indexOf("not unique") >= 0 && s.indexOf("not identifiers") >= 0
        }

        // The non-vacuity guard is on the SCREEN's corpus, not the column's.
        //
        // An earlier draft floored `apparatusTexts.length` instead, and that was
        // wrong in the way this whole finding is about: once `drop-apparatus`
        // removes the column the walk correctly finds nothing there, and a floor
        // would have failed a legitimate merge — the inversion again, one level
        // out. What must not go vacuous is the screen-wide sweep, because that
        // is what the carrier count is drawn from.
        var all = spec.everyTextOn(screen)
        verify(all.length > 15,
               "the screen sweep must be finding copy, got " + all.length
               + " — a neutered walk makes every count below meaningless")

        // Deliberately NOT asserted: that the margin note also carries it.
        // Requiring that is the inversion this finding is about — it would make
        // the margin copy undroppable, which is the opposite of "repetition is
        // permitted, not required". This count is subtracted, never floored.
        var inApparatus = 0
        for (var i = 0; i < apparatusTexts.length; i++) {
            if (carriesIt(apparatusTexts[i]))
                inApparatus++
        }

        var total = 0
        for (var j = 0; j < all.length; j++) {
            if (carriesIt(all[j]))
                total++
        }

        // The assertion. At least one carrier outside the apparatus column —
        // never an exact total, so removing the column is permitted and
        // removing the body copy is not.
        verify(total - inApparatus >= 1,
               "the obligation must be stated somewhere OUTSIDE the apparatus "
               + "column: that column is annotation, removable by a change with "
               + "no reason to read this spec, so an obligation resting on it "
               + "alone is deleted as a side effect of dropping decoration. "
               + "Found " + total + " carrier(s), " + inApparatus + " of them in "
               + "apparatus. Repetition in the margin is permitted, not "
               + "required — do not satisfy this by pinning a count.")
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

    function test_every_row_carries_a_mark_drawn_from_its_own_address() {
        // Spec/test review finding: the mark was pinned NOWHERE. Deleting the
        // `Identicon` from every candidate row passed 49/49, and feeding every
        // row the same constant address passed 49/49. The requirement is titled
        // "A candidate row shows the full address **and the mark**", and its
        // scenario says "each mark is derived from THAT ROW's address rather
        // than from a shared or fixed value" — both halves were unasserted.
        // `tst_identicon.qml` tests the component in isolation and says nothing
        // about the row.
        //
        // Two assertions, because the two mutations are different defects: a
        // missing mark removes the second recognition channel, and a shared
        // mark replaces it with one that actively misleads — every candidate
        // looking alike is worse than no mark, since a user told the shape
        // identifies a key sees five identical shapes and concludes they are
        // interchangeable.
        var addrOne = "44".repeat(32)
        var addrTwo = "55".repeat(32)
        var screen = makeScreen({
            "generate_identity_slate":
                '{"slate":"s1","count":2,"candidates":['
                + '{"index":0,"path":7,"address":"' + addrOne + '","publicKey":"pk"},'
                + '{"index":1,"path":8,"address":"' + addrTwo + '","publicKey":"qk"}]}'
        })
        screen.requestSlate()

        var rows = spec.candidateRowsOn(screen)
        compare(rows.length, 2, "the fixture's own precondition: two rows")

        var addresses = [addrOne, addrTwo]
        var signatures = []
        for (var r = 0; r < rows.length; r++) {
            var marks = spec.marksOn(rows[r])
            // Both bounds on the walker. Zero is the deletion mutation; more
            // than one would mean a row carrying a second mark, which is its own
            // confusion about which one identifies the key.
            compare(marks.length, 1,
                    "row " + r + " must carry exactly one mark — deleting the "
                    + "Identicon from the row is invisible to every text sweep, "
                    + "because a mark is not a Text")
            compare(String(marks[0].address), addresses[r],
                    "row " + r + "'s mark must be drawn from THAT ROW's address, "
                    + "not a shared or fixed value")
            signatures.push(spec.markSignature(marks[0]))
        }

        // And the two marks actually DRAW differently. The address check above
        // would pass on a mark bound to the right address that derived its form
        // and inks from something else; this reads the eight selectors
        // Identicon computes and requires the tuples to differ.
        verify(signatures[0] !== signatures[1],
               "two candidates with different addresses must produce different "
               + "marks — a mark identical across every row is a recognition "
               + "channel that distinguishes nothing, got " + signatures[0]
               + " for both")
        screen.destroy()
    }

    function test_a_row_shows_its_address_and_its_mark_and_nothing_else() {
        // Review finding (correctness): this was a BLOCKLIST over "7", "0",
        // "1", "#1" and the public key, and a blocklist cannot catch a
        // name-shaped value nobody anticipated. The reviewer measured it: a
        // `Text` reading "Key H", derived from the fixture's `path:7`, passed
        // all 38 tests including this one, because "Key H" is on no list. The
        // blunter `String(path)` mutation was caught only by the coincidence
        // that the fixture's path is 7 and "7" happened to be listed.
        //
        // The spec states the property structurally — "The row SHALL leave the
        // name unshown rather than substituted" — and the row is built so its
        // only text is the address, plus SELECTED on the chosen one. So assert
        // the SET, not a list of exclusions: any value added in a name's
        // position then fails whether or not its spelling was anticipated, and
        // when the generated name lands, the expected set gains a member
        // deliberately rather than a blocklist silently admitting one.
        var addrOne = "44".repeat(32)
        var addrTwo = "55".repeat(32)
        var screen = makeScreen({
            "generate_identity_slate":
                '{"slate":"s1","count":2,"candidates":['
                + '{"index":0,"path":7,"address":"' + addrOne + '","publicKey":"pk"},'
                + '{"index":1,"path":8,"address":"' + addrTwo + '","publicKey":"qk"}]}'
        })
        screen.requestSlate()
        screen.select(0)

        var rows = spec.candidateRowsOn(screen)
        // The walker's UPPER bound as well as its lower one. Narrowed to
        // nothing, it would satisfy every per-row assertion below vacuously;
        // widened past the delegates it would drag the rest of the screen's
        // copy in and fail against correct code. Two candidates, two rows.
        compare(rows.length, 2,
                "the row walker must find exactly the rows the reply carried")

        // The addresses THIS TEST wrote into the reply, in the order it wrote
        // them — compared against what each row renders, so a row showing the
        // wrong candidate's address fails here too.
        var expected = [[addrOne, "SELECTED"], [addrTwo]]
        for (var r = 0; r < rows.length; r++) {
            var shown = spec.visibleTextsOn(rows[r]).slice().sort()
            var want = expected[r].slice().sort()
            compare(shown.join(" | "), want.join(" | "),
                    "row " + r + " may show its address and, when chosen, the word "
                    + "SELECTED — and nothing else. Any further string is a value "
                    + "standing where the generated name will go, which is read as "
                    + "the thing being chosen and is not it.")
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

    // `everyTextOn` deliberately IGNORES `visible`, which is what makes it the
    // right instrument for "this string appears nowhere in the file" and the
    // WRONG one for "the screen currently says this". A phase-gated sentence is
    // present in the tree in every phase; only `visible` up the chain decides
    // whether a user reads it. So every assertion about what the screen states
    // NOW goes through this one instead.
    function visibleTextsOn(item) {
        var items = spec.everyTextItemOn(item)
        var texts = []
        for (var i = 0; i < items.length; i++) {
            if (items[i].text === undefined || String(items[i].text) === "")
                continue
            if (spec.isShown(items[i]))
                texts.push(String(items[i].text))
        }
        return texts
    }

    // The one visible string containing `needle`, or "" if none. Returns the
    // string rather than a boolean so a caller can assert what it says as well
    // as that it is there — and fails loudly on two matches, because an
    // assertion written against "the" sentence must not silently pick one of a
    // pair.
    function visibleTextMatching(item, needle) {
        var texts = spec.visibleTextsOn(item)
        var hits = []
        for (var i = 0; i < texts.length; i++) {
            if (texts[i].indexOf(needle) >= 0)
                hits.push(texts[i])
        }
        compare(hits.length, 1,
                "expected exactly one visible string containing \"" + needle
                + "\", got " + hits.length + ": " + JSON.stringify(hits))
        return hits[0]
    }

    // Every candidate row currently built by the Repeater.
    //
    // Identified by carrying BOTH `chosen` and `modelData`: `modelData` alone
    // is not enough, because a delegate's children inherit the context property
    // and a helper narrowed by one loose property has already excluded the
    // wrong subtree once in this repo. Callers pin the count as well as the
    // contents — a walker that finds nothing passes an "each row shows only X"
    // assertion just as well as a correct one does.
    function candidateRowsOn(item) {
        var found = []
        spec.collectRows(item, found)
        return found
    }

    function collectRows(item, out) {
        if (item === null || item === undefined)
            return
        if (item.chosen !== undefined && item.modelData !== undefined)
            out.push(item)
        var kids = item.children
        if (kids === undefined)
            return
        for (var i = 0; i < kids.length; i++)
            spec.collectRows(kids[i], out)
    }

    // Every mark (Identicon) in a subtree.
    //
    // `visibleTextsOn()` CANNOT see a mark — a mark is not a `Text` — which is
    // why the row-contents test passed with the Identicon deleted. This is the
    // different instrument that needs, not a wider text sweep.
    //
    // Identified by `address` AND two of the mark's own selector functions.
    // `address` alone also matches `AddressLabel`, which would make a row look
    // marked when it only shows its address — the precise confusion the spec's
    // "a second recognition channel, not a second guarantee" depends on not
    // making. Callers pin the count found, both bounds.
    function marksOn(item) {
        var found = []
        spec.collectMarks(item, found)
        return found
    }

    function collectMarks(item, out) {
        if (item === null || item === undefined)
            return
        if (item.address !== undefined && typeof item._form === "function"
            && typeof item._inkA === "function")
            out.push(item)
        var kids = item.children
        if (kids === undefined)
            return
        for (var i = 0; i < kids.length; i++)
            spec.collectMarks(kids[i], out)
    }

    // What a mark actually draws, as a comparable tuple: the eight selectors
    // Identicon derives from the address. Compared rather than the `address`
    // property alone, because a mark bound to the right address that drew from
    // something else would satisfy an address check and still show every
    // candidate the same shape.
    function markSignature(mark) {
        return [mark._form(), mark._inkA(), mark._inkB(), mark._outlineInk(),
                mark._angleDeg(), mark._pitch(), mark._duty(), mark._weave()].join("/")
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
