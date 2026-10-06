import QtQuick
import QtTest
import "../src/qml"

// `composer-view`, "A publish outcome is displayed only on the visit in which
// the publish was made".
//
// **These tests drive `Main.qml`, not a composer**, because the defect lives in
// the navigator's shape rather than in the composer. `FeedScreen` and
// `DThreadScreen` are each mounted once and shown or hidden by a `visible:`
// binding, so a composer's outcome survived leaving the screen and was rendered
// again on the next visit. A test instantiating `DComposer` on its own builds a
// fresh composer per test and cannot see that at all.
//
// **Every absence assertion below is preceded by the presence it withdraws.**
// "No outcome is displayed" passes trivially on a composer that never displayed
// one, so each test first shows the outcome on screen during the visit that
// produced it, and only then leaves and returns.
TestCase {
    id: spec
    name: "PublishOutcomeVisits"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    Component { id: mainComponent; Main {} }

    readonly property string stoaA:
        "aaaaaaaa11111111111111111111111111111111111111111111111111111111"
    readonly property string stoaB:
        "bbbbbbbb22222222222222222222222222222222222222222222222222222222"
    readonly property string keyA:
        "k:0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
    readonly property string rootOp: "cc" + "11".repeat(31)

    // A feed row heading the thread `rootOp`, in the shape `wire.rs` pins.
    function rowFor(op, text) {
        return { thread: op, currentVersion: op, author: spec.keyA,
                 body: { text: text, removed: 0, marked: 0 },
                 attachments: [], isRevised: false, isHidden: false }
    }

    // A fake forum with posting open. It RECORDS every call, and its answers
    // depend on what was asked and on what has been published:
    //
    //   - `publish_post` answers with `postReply`, which a test sets to the
    //     outcome it wants, and a newly stored post becomes a row the next
    //     `list_threads` returns — so "the re-read returns a row for it" is the
    //     fake's consequence of the publish, not a fixture the test hard-wired.
    //   - `list_threads` answers the error shape while `threadsFail` is set.
    function forumBridge() {
        return {
            calls: [],
            rows: [spec.rowFor(spec.rootOp, "the thread's root")],
            postReply: '{"opId":"newpost","wasNew":true}',
            threadsFail: false,
            callModule: function (module, method, args) {
                this.calls.push({ method: method, args: args })
                if (method === "list_stoas")
                    return JSON.stringify({ items: [
                        { stoa: spec.stoaA, foundingTitle: "Nym Research" }
                    ], page: 0, hasMore: false })
                if (method === "get_capabilities")
                    return '{"canPost":true,"reason":""}'
                if (method === "who_am_i")
                    return '{"hasIdentity":true,"publicKey":"' + spec.keyA
                           + '","recoveryNeedsTheRecord":false}'
                if (method === "list_threads") {
                    if (this.threadsFail)
                        return '{"error":"the store could not be read"}'
                    return JSON.stringify({ items: this.rows, page: 0, hasMore: false })
                }
                if (method === "read_thread")
                    return JSON.stringify({ items: [{
                        thread: spec.rootOp, id: spec.rootOp, currentVersion: spec.rootOp,
                        author: spec.keyA, isRevised: false,
                        moderation: { state: "unmoderated" }, position: "0",
                        body: { text: "the thread's root", removed: 0, marked: 0 }
                    }], page: 0, hasMore: false })
                if (method === "publish_post") {
                    var sent = JSON.parse(String(args[0]))
                    var answer = JSON.parse(this.postReply)
                    if (answer.wasNew === true)
                        this.rows = [spec.rowFor(answer.opId, sent.body)].concat(this.rows)
                    return this.postReply
                }
                if (method === "publish_reply")
                    return '{"opId":"newreply","wasNew":true}'
                return '{"error":"no fake reply for ' + method + '"}'
            }
        }
    }

    function argsOfLast(bridge, method) {
        for (var i = bridge.calls.length - 1; i >= 0; i--)
            if (bridge.calls[i].method === method)
                return JSON.parse(String(bridge.calls[i].args[0]))
        return null
    }

    // Visibility checked on every ancestor, as `tst_navigation.qml` does: an
    // element inside a hidden screen must not count as displayed.
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

    // Every displayed text containing `phrase`, so a message is caught by what
    // it SAYS as well as by which element carries it.
    function visibleTextsContaining(item, phrase) {
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && typeof node.text === "string" && node.text.indexOf(phrase) >= 0)
                found.push(node.text)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    function outcomesShown(view, kind) {
        return spec.visibleNamed(view, kind + "OutcomeMessage").length
    }

    // Through the controls a user acts on: the draft field and the submit
    // button, each found only where it is displayed.
    function publish(view, kind, text) {
        var field = spec.visibleNamed(view, kind + "DraftField")
        compare(field.length, 1, "the " + kind + " composer is on screen")
        field[0].text = text
        var submit = spec.visibleNamed(view, kind + "SubmitButton")
        compare(submit.length, 1, "with its submit control")
        submit[0].clicked()
    }

    // The feed's "All Stoas", then the list's "Open" on the one Stoa it holds.
    function reopenFromTheList(view) {
        spec.visibleNamed(view, "feedBackButton")[0].clicked()
        compare(view.screenShown, "list")
        var open = spec.visibleNamed(view, "openStoaButton")
        compare(open.length, 1, "the list offers its one Stoa")
        open[0].clicked()
        compare(view.screenShown, "feed")
        compare(view.chosen.stoa, spec.stoaA, "and it is the same Stoa")
    }

    // The thread link's own target, raised through the feed's `threadOpened`,
    // as `tst_navigation.qml` does. Its `MouseArea` takes a real click only in
    // a window (`tst_feed_mouse_clicks.qml`), and which row's thread opens is
    // the row's judgement either way.
    function openTheThread(view) {
        var link = spec.visibleNamed(view, "readThreadLink")
        verify(link.length > 0, "a row offers its thread")
        var root = link[link.length - 1].target
        compare(root, spec.rootOp, "the fixture's root row is the one opened")
        spec.visibleNamed(view, "feed")[0].threadOpened(root)
        compare(view.screenShown, "thread")
    }

    function openedOnStoaA() {
        var bridge = spec.forumBridge()
        Core.bridge = bridge
        var view = mainComponent.createObject(null, {})
        spec.visibleNamed(view, "openStoaButton")[0].clicked()
        compare(view.screenShown, "feed")
        compare(view.feedCanPost, true, "posting is open, so the composer is shown")
        return { view: view, bridge: bridge }
    }

    // ---- the scenarios ---------------------------------------------------

    // Scenario: A confirmation is gone after reopening the Stoa from the list.
    // The issue's reproduction, through the same two controls.
    function test_a_confirmation_is_gone_after_reopening_the_stoa_from_the_list() {
        var s = spec.openedOnStoaA()
        spec.publish(s.view, "post", "first post")
        compare(spec.outcomesShown(s.view, "post"), 1,
                "the confirmation is displayed on the visit that produced it")
        verify(spec.visibleTextsContaining(s.view, "saved on this machine").length > 0)

        spec.reopenFromTheList(s.view)

        compare(spec.outcomesShown(s.view, "post"), 0,
                "a later visit displays no outcome until its own composer reports one")
        compare(spec.visibleTextsContaining(s.view, "saved on this machine"), [],
                "and nothing claims the content was saved on this machine")
        s.view.destroy()
    }

    // Scenario: A confirmation is gone after returning from a thread.
    function test_a_confirmation_is_gone_after_returning_from_a_thread() {
        var s = spec.openedOnStoaA()
        spec.publish(s.view, "post", "first post")
        compare(spec.outcomesShown(s.view, "post"), 1)

        spec.openTheThread(s.view)
        spec.visibleNamed(s.view, "threadBackButton")[0].clicked()
        compare(s.view.screenShown, "feed")

        compare(spec.outcomesShown(s.view, "post"), 0,
                "returning from a thread begins a new visit to the feed")
        s.view.destroy()
    }

    // Scenario: Every kind of outcome is gone on the next visit. The stored
    // case is the first test; these are the other two.
    function test_an_already_published_or_refused_outcome_is_gone_on_the_next_visit() {
        var cases = [
            { label: "already published", reply: '{"opId":"newpost","wasNew":false}',
              says: "already published" },
            { label: "refused", reply: '{"error":"the keystore is readable by others"}',
              says: "was not published" }
        ]
        for (var i = 0; i < cases.length; i++) {
            var s = spec.openedOnStoaA()
            s.bridge.postReply = cases[i].reply
            spec.publish(s.view, "post", "a post")
            compare(spec.outcomesShown(s.view, "post"), 1,
                    cases[i].label + ": displayed on the visit that produced it")
            verify(spec.visibleTextsContaining(s.view, cases[i].says).length > 0,
                   cases[i].label + ": and it says so")

            spec.reopenFromTheList(s.view)

            compare(spec.outcomesShown(s.view, "post"), 0,
                    cases[i].label + ": gone on the next visit")
            compare(spec.visibleTextsContaining(s.view, cases[i].says), [],
                    cases[i].label + ": nothing on screen still says it")
            s.view.destroy()
        }
    }

    // Scenario: An outcome does not follow the user into another Stoa.
    //
    // Opened through `open()`, which is what the list's `stoaChosen` handler
    // calls: the fake's listing holds one Stoa so that `reopenFromTheList` is
    // unambiguous, and B is reached by the navigator's own transition.
    function test_an_outcome_does_not_follow_the_user_into_another_stoa() {
        var s = spec.openedOnStoaA()
        spec.publish(s.view, "post", "posted in A")
        compare(spec.outcomesShown(s.view, "post"), 1)

        spec.visibleNamed(s.view, "feedBackButton")[0].clicked()
        s.view.open(spec.stoaB, "Another Stoa", "")
        compare(s.view.screenShown, "feed")
        compare(s.view.chosen.stoa, spec.stoaB)

        compare(spec.outcomesShown(s.view, "post"), 0,
                "B's feed displays no outcome of a publish made in A")
        s.view.destroy()
    }

    // Scenario: A failed read on the later visit carries no outcome. The issue
    // saw the confirmation rendered over the read-failure panel.
    function test_a_failed_read_on_the_later_visit_carries_no_outcome() {
        var s = spec.openedOnStoaA()
        spec.publish(s.view, "post", "first post")
        compare(spec.outcomesShown(s.view, "post"), 1)

        s.bridge.threadsFail = true
        spec.reopenFromTheList(s.view)

        compare(s.view.feedReadState, "failed", "the feed is in its failed state")
        compare(s.view.feedCanPost, true,
                "with posting still open, so the composer is on screen and could "
                + "show an outcome — which is the case the issue saw")
        compare(spec.outcomesShown(s.view, "post"), 0,
                "and it displays none")
        s.view.destroy()
    }

    // Scenario: A reply's outcome is gone on the next visit to the thread.
    function test_a_replys_outcome_is_gone_on_the_next_visit_to_the_thread() {
        var s = spec.openedOnStoaA()
        spec.openTheThread(s.view)
        spec.publish(s.view, "reply", "a reply")
        compare(spec.outcomesShown(s.view, "reply"), 1,
                "the reply's outcome is displayed on the visit that produced it")

        spec.visibleNamed(s.view, "threadBackButton")[0].clicked()
        compare(s.view.screenShown, "feed")
        spec.openTheThread(s.view)
        compare(s.view.reading.rootOp, spec.rootOp, "the same thread")

        compare(spec.outcomesShown(s.view, "reply"), 0,
                "the thread screen's composer displays no outcome")
        s.view.destroy()
    }

    // Scenario: The outcome stays for the rest of the visit that produced it.
    //
    // The guard against over-clearing: a fix that cleared the outcome on every
    // read would pass every test above and fail this one, because a newly
    // stored post is followed by a re-read.
    function test_the_outcome_stays_across_the_re_read_that_follows_the_publish() {
        var s = spec.openedOnStoaA()
        var readsBefore = 0
        for (var i = 0; i < s.bridge.calls.length; i++)
            if (s.bridge.calls[i].method === "list_threads")
                readsBefore++

        spec.publish(s.view, "post", "first post")

        var readsAfter = 0
        for (var j = 0; j < s.bridge.calls.length; j++)
            if (s.bridge.calls[j].method === "list_threads")
                readsAfter++
        compare(readsAfter, readsBefore + 1, "the publish was followed by a re-read")
        compare(s.view.feedRowCount, 2, "and that re-read returned a row for the post")

        verify(spec.visibleTextsContaining(s.view, "Your post was saved on this machine.").length > 0,
               "the message for a newly stored op is still displayed")
        s.view.destroy()
    }

    // Scenario: A publish on the later visit displays its own outcome.
    function test_a_publish_on_the_later_visit_displays_its_own_outcome() {
        var s = spec.openedOnStoaA()
        spec.publish(s.view, "post", "first post")
        compare(spec.outcomesShown(s.view, "post"), 1)

        spec.reopenFromTheList(s.view)
        s.bridge.postReply = '{"error":"the keystore is readable by others"}'
        spec.publish(s.view, "post", "second post")

        verify(spec.visibleTextsContaining(s.view, "Your post was not published.").length > 0,
               "the feed's composer displays the refusal")
        compare(spec.visibleTextsContaining(s.view, "saved on this machine"), [],
                "and not the message for a newly stored op")
        s.view.destroy()
    }

    // ---- the draft, which the requirement leaves undecided ----------------

    // NO SPEC: `composer-view` does not decide whether an unsubmitted draft
    // survives leaving the screen. This change clears only the outcome, so the
    // draft is kept: the smallest change meeting the requirement, and the
    // behaviour the view already had. `design.md` records the alternative.
    function test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened() {
        var s = spec.openedOnStoaA()
        spec.visibleNamed(s.view, "postDraftField")[0].text = "half-written"

        spec.reopenFromTheList(s.view)

        compare(spec.visibleNamed(s.view, "postDraftField")[0].text, "half-written",
                "the draft typed before leaving is still in the field")
        s.view.destroy()
    }

    // NO SPEC, and very probably a defect rather than a choice: the feed's
    // composer is the same instance for every Stoa, so a draft typed in one
    // Stoa is still in the field when another is opened, and submitting it
    // publishes it to the Stoa now open. This test pins what the view does
    // today so the decision is visible; it was not widened into this change,
    // and the spec does not yet say which Stoa a draft belongs to. A fix flips
    // both assertions.
    function test_a_draft_typed_in_one_stoa_is_still_held_and_published_in_another() {
        var s = spec.openedOnStoaA()
        spec.visibleNamed(s.view, "postDraftField")[0].text = "meant for A"

        spec.visibleNamed(s.view, "feedBackButton")[0].clicked()
        s.view.open(spec.stoaB, "Another Stoa", "")
        compare(s.view.chosen.stoa, spec.stoaB)

        compare(spec.visibleNamed(s.view, "postDraftField")[0].text, "meant for A",
                "the draft typed in A is in B's composer")
        spec.visibleNamed(s.view, "postSubmitButton")[0].clicked()
        var sent = spec.argsOfLast(s.bridge, "publish_post")
        verify(sent !== null, "submitting reached publish_post")
        compare(sent.stoa, spec.stoaB, "and it was published to B")
        compare(sent.body, "meant for A")
        s.view.destroy()
    }
}
