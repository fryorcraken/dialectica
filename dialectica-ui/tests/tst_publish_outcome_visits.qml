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
    readonly property string otherRootOp: "dd" + "22".repeat(31)

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
    //   - `list_threads` answers the error shape while `threadsFail` is set, the
    //     verbatim `threadsReply` while that is set, and otherwise the rows,
    //     reporting `hasMoreThreads` as its `hasMore`.
    //   - `read_thread` answers the error shape while `readThreadFail` is set.
    //   - `publish_reply` answers with `replyReply`, as `publish_post` does with
    //     `postReply`.
    function forumBridge() {
        return {
            calls: [],
            rows: [spec.rowFor(spec.rootOp, "the thread's root")],
            postReply: '{"opId":"newpost","wasNew":true}',
            replyReply: '{"opId":"newreply","wasNew":true}',
            threadsFail: false,
            threadsReply: "",
            hasMoreThreads: false,
            readThreadFail: false,
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
                    if (this.threadsReply !== "")
                        return this.threadsReply
                    return JSON.stringify({ items: this.rows, page: 0,
                                            hasMore: this.hasMoreThreads })
                }
                if (method === "read_thread") {
                    if (this.readThreadFail)
                        return '{"error":"that thread could not be read"}'
                    return JSON.stringify({ items: [{
                        thread: spec.rootOp, id: spec.rootOp, currentVersion: spec.rootOp,
                        author: spec.keyA, isRevised: false,
                        moderation: { state: "unmoderated" }, position: "0",
                        body: { text: "the thread's root", removed: 0, marked: 0 }
                    }], page: 0, hasMore: false })
                }
                if (method === "publish_post") {
                    var sent = JSON.parse(String(args[0]))
                    var answer = JSON.parse(this.postReply)
                    if (answer.wasNew === true)
                        this.rows = [spec.rowFor(answer.opId, sent.body)].concat(this.rows)
                    return this.postReply
                }
                if (method === "publish_reply")
                    return this.replyReply
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

    // **The one place "displayed" is defined for this file.** Every node under
    // `item` that `matches(node)` AND whose own `visible` and every ancestor's
    // are not false, as `tst_navigation.qml` checks it: an element inside a
    // hidden screen must not count as displayed. The finders below differ only in
    // the predicate they hand in, so a change to what counts as displayed (a
    // zero `opacity`, say) is made here and reaches all of them.
    function visibleNodes(item, matches) {
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && matches(node))
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    function visibleNamed(item, name) {
        return spec.visibleNodes(item, function (node) {
            return node.objectName === name
        })
    }

    // Every displayed text containing `phrase`, so a message is caught by what
    // it SAYS as well as by which element carries it.
    function visibleTextsContaining(item, phrase) {
        return spec.visibleNodes(item, function (node) {
            return typeof node.text === "string" && node.text.indexOf(phrase) >= 0
        }).map(function (node) {
            return node.text
        })
    }

    // Whether the message for a newly stored op is on screen, by what it says.
    // `kind` is "post" or "reply", the word the message uses for what was stored.
    function storedMessageShown(view, kind) {
        return spec.visibleTextsContaining(
            view, "Your " + kind + " was saved on this machine.").length > 0
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

    // Open a thread by raising the feed's `threadOpened`, as `tst_navigation.qml`
    // does: the thread link's `MouseArea` takes a real click only in a window
    // (`tst_feed_mouse_clicks.qml`), and which row's thread opens is the row's
    // judgement either way. **Both ways to name the thread are this one helper.**
    //
    //   - Called with no `root`, the target is read off the rendered row's own
    //     `readThreadLink`, and asserted to be the fixture's `rootOp`: the thread
    //     opened is the one the row offers.
    //   - Called with a `root`, that root is opened, which is the only way to
    //     reach a thread the fixture's one row does not offer.
    //
    // Either way it asserts the screen shown is the thread, and that it is the
    // thread asked for.
    function openTheThread(view, root) {
        if (root === undefined) {
            var link = spec.visibleNamed(view, "readThreadLink")
            verify(link.length > 0, "a row offers its thread")
            root = link[link.length - 1].target
            compare(root, spec.rootOp, "the fixture's root row is the one opened")
        }
        spec.visibleNamed(view, "feed")[0].threadOpened(root)
        compare(view.screenShown, "thread")
        compare(view.reading.rootOp, root, "and it is the thread asked for")
    }

    // A displayed element whose own `text` is exactly `text`: how a control
    // with no `objectName` ("Next", "Try reading again", "SHOW HIDDEN") is found
    // by what a user reads on it.
    function visibleWithText(item, text) {
        return spec.visibleNodes(item, function (node) {
            return node.text === text
        })
    }

    // Press the one displayed button reading `text`. A `FlatButton` and the
    // label inside it both carry the text, so only what can be clicked counts.
    function pressButtonReading(view, text) {
        var buttons = spec.visibleWithText(view, text).filter(function (node) {
            return typeof node.clicked === "function"
        })
        compare(buttons.length, 1, "exactly one displayed control reads '" + text + "'")
        buttons[0].clicked()
    }

    // The "SHOW HIDDEN" toggle is a `Text` whose only child is the `MouseArea`
    // that carries the handler, and the handler reads nothing from the event.
    function toggleShowHidden(view) {
        var labels = spec.visibleWithText(view, "SHOW HIDDEN")
        compare(labels.length, 1, "the displayed screen offers one toggle")
        compare(labels[0].children.length, 1, "carried by one MouseArea")
        labels[0].children[0].clicked(null)
    }

    function countCalls(bridge, method) {
        var n = 0
        for (var i = 0; i < bridge.calls.length; i++)
            if (bridge.calls[i].method === method)
                n++
        return n
    }

    // The thread's back button, which begins a new visit to the feed.
    function backToTheFeed(view) {
        spec.visibleNamed(view, "threadBackButton")[0].clicked()
        compare(view.screenShown, "feed")
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
        spec.backToTheFeed(s.view)

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

        spec.backToTheFeed(s.view)
        spec.openTheThread(s.view)

        compare(spec.outcomesShown(s.view, "reply"), 0,
                "the thread screen's composer displays no outcome")
        s.view.destroy()
    }

    // Scenario: The outcome stays for the rest of the visit that produced it.
    //
    // The scenario's own test, and **not the only guard against over-clearing**.
    // A fix that cleared the outcome on every read (`composer.clearOutcome()`
    // first in `FeedScreen.reload()`) turns red every feed test in this file
    // that publishes and then asserts the outcome displayed, this one among
    // them, because a newly stored post is always followed by a re-read. The
    // absence tests above go red too, at the presence assertion each makes
    // before it leaves. Measured by making that edit and running this file; it is
    // why those presence assertions are not decoration and are not to be dropped.
    // What this test pins that they do not: that the re-read happened and
    // returned a row for the post, which is the scenario's own condition.
    function test_the_outcome_stays_across_the_re_read_that_follows_the_publish() {
        var s = spec.openedOnStoaA()
        var readsBefore = spec.countCalls(s.bridge, "list_threads")

        spec.publish(s.view, "post", "first post")

        compare(spec.countCalls(s.bridge, "list_threads"), readsBefore + 1,
                "the publish was followed by a re-read")
        compare(s.view.feedRowCount, 2, "and that re-read returned a row for the post")

        verify(spec.storedMessageShown(s.view, "post"),
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

    // ---- the rest of the requirement ---------------------------------------
    //
    // What follows pins the requirement's later scenarios and the clauses of its
    // prose that no scenario states. A test pinning a scenario carries a
    // `// Scenario:` line naming it; one carrying a `// Requirement:` line pins a
    // quoted clause of the prose instead. Neither kind is an extra beyond the
    // spec, so do not trim one as redundant. The prose names "whatever that
    // re-read returns", "a failed read included", every composer the view
    // mounts, a different thread, a later publish replacing an outcome, and the
    // rule that re-reading, retrying, paging and changing what a screen lists
    // (hidden content included or excluded) do not begin a visit.

    // Requirement: "its outcome MUST remain displayed across the re-read that
    // follows a successful publish, whatever that re-read returns". The
    // scenario above has the re-read return a row; these are the others. A fix
    // that cleared the outcome on a failed or empty read passes that scenario.
    function test_the_outcome_stays_whatever_the_re_read_returns() {
        var stored = '{"opId":"newpost","wasNew":true}'
        var existing = '{"opId":"newpost","wasNew":false}'
        var cases = [
            { label: "newly stored, then the read fails",
              post: stored, set: { threadsFail: true },
              state: "failed", says: "Your post was saved on this machine." },
            { label: "already published, then the read fails",
              post: existing, set: { threadsFail: true },
              state: "failed", says: "This post was already published." },
            { label: "newly stored, then an empty page",
              post: stored,
              set: { threadsReply: '{"items":[],"page":0,"hasMore":false}' },
              state: "ok", says: "Your post was saved on this machine." },
            { label: "newly stored, then an answer with no items",
              post: stored, set: { threadsReply: '{"page":0,"hasMore":false}' },
              state: "failed", says: "Your post was saved on this machine." }
        ]
        for (var i = 0; i < cases.length; i++) {
            var s = spec.openedOnStoaA()
            s.bridge.postReply = cases[i].post
            for (var key in cases[i].set)
                s.bridge[key] = cases[i].set[key]
            var readsBefore = spec.countCalls(s.bridge, "list_threads")

            spec.publish(s.view, "post", "a post")

            compare(spec.countCalls(s.bridge, "list_threads"), readsBefore + 1,
                    cases[i].label + ": the publish was followed by a re-read")
            compare(s.view.feedReadState, cases[i].state,
                    cases[i].label + ": and the re-read reached the state under test")
            compare(spec.outcomesShown(s.view, "post"), 1,
                    cases[i].label + ": the outcome is still displayed")
            verify(spec.visibleTextsContaining(s.view, cases[i].says).length > 0,
                   cases[i].label + ": and it is the one the publish reported")
            s.view.destroy()
        }
    }

    // Scenario: A reply's outcome is displayed again when a failed re-read
    // recovers.
    //
    // The same, for the reply composer, where a re-read that fails also removes
    // the composer from the screen (it is shown only on a successful read). So
    // the outcome cannot be seen during the failure: it must be there again
    // when the read recovers, on the same visit.
    function test_a_replys_outcome_stays_across_a_failed_re_read_of_the_thread() {
        var s = spec.openedOnStoaA()
        spec.openTheThread(s.view)
        s.bridge.readThreadFail = true

        spec.publish(s.view, "reply", "a reply")
        compare(s.view.threadReadState, "failed", "the re-read after the publish failed")
        compare(spec.visibleNamed(s.view, "replyDraftField").length, 0,
                "so the reply composer is not on screen")

        s.bridge.readThreadFail = false
        spec.pressButtonReading(s.view, "Try reading again")
        compare(s.view.threadReadState, "ok", "the read recovered, on the same visit")

        compare(spec.outcomesShown(s.view, "reply"), 1,
                "the outcome of the reply published on this visit is displayed")
        verify(spec.visibleTextsContaining(s.view, "Your reply was saved on this machine.").length > 0)
        s.view.destroy()
    }

    // Scenario: A reply's outcome does not follow the user into another thread.
    // Requirement: "This holds whether the later visit is for the same Stoa or
    // thread or a different one."
    function test_a_replys_outcome_does_not_follow_the_user_into_another_thread() {
        var s = spec.openedOnStoaA()
        spec.openTheThread(s.view, spec.rootOp)
        spec.publish(s.view, "reply", "a reply")
        compare(spec.outcomesShown(s.view, "reply"), 1)

        spec.backToTheFeed(s.view)
        spec.openTheThread(s.view, spec.otherRootOp)

        compare(spec.visibleNamed(s.view, "replyDraftField").length, 1,
                "the other thread's reply composer is on screen")
        compare(spec.outcomesShown(s.view, "reply"), 0,
                "and displays no outcome of a reply made to the first thread")
        s.view.destroy()
    }

    // Scenario: A failed read on the later visit to the thread carries no
    // outcome once it recovers.
    // Requirement: "This holds ... whatever state that visit's read reaches, a
    // failed read included", and it applies to the reply composer too. On the
    // thread screen a failed read hides the composer, so absence DURING the
    // failure proves nothing: the outcome must be absent once the read recovers
    // and the composer is back, on the visit that never published.
    function test_a_failed_read_on_the_later_visit_to_the_thread_carries_no_outcome() {
        var s = spec.openedOnStoaA()
        spec.openTheThread(s.view)
        spec.publish(s.view, "reply", "a reply")
        compare(spec.outcomesShown(s.view, "reply"), 1)
        spec.backToTheFeed(s.view)

        s.bridge.readThreadFail = true
        spec.openTheThread(s.view, spec.rootOp)
        compare(s.view.threadReadState, "failed", "the later visit's read failed")
        compare(spec.outcomesShown(s.view, "reply"), 0)

        s.bridge.readThreadFail = false
        spec.pressButtonReading(s.view, "Try reading again")
        compare(s.view.threadReadState, "ok")
        compare(spec.visibleNamed(s.view, "replyDraftField").length, 1,
                "the composer is back on screen, so an outcome could be shown")
        compare(spec.outcomesShown(s.view, "reply"), 0,
                "and it shows none: nothing was published on this visit")
        compare(spec.visibleTextsContaining(s.view, "saved on this machine"), [])
        s.view.destroy()
    }

    // Scenario: Every kind of reply outcome is gone on the next visit to the
    // thread. The feed's counterpart is "Every kind of outcome is gone on the
    // next visit", and the requirement applies to every composer the view
    // mounts.
    function test_every_kind_of_reply_outcome_is_gone_on_the_next_visit_to_the_thread() {
        var cases = [
            { label: "already published", reply: '{"opId":"newreply","wasNew":false}',
              says: "already published" },
            { label: "refused", reply: '{"error":"the parent has not reached this peer"}',
              says: "was not published" }
        ]
        for (var i = 0; i < cases.length; i++) {
            var s = spec.openedOnStoaA()
            s.bridge.replyReply = cases[i].reply
            spec.openTheThread(s.view)
            spec.publish(s.view, "reply", "a reply")
            compare(spec.outcomesShown(s.view, "reply"), 1,
                    cases[i].label + ": displayed on the visit that produced it")
            verify(spec.visibleTextsContaining(s.view, cases[i].says).length > 0,
                   cases[i].label + ": and it says so")

            spec.backToTheFeed(s.view)
            spec.openTheThread(s.view)

            compare(spec.outcomesShown(s.view, "reply"), 0,
                    cases[i].label + ": gone on the next visit")
            compare(spec.visibleTextsContaining(s.view, cases[i].says), [],
                    cases[i].label + ": nothing on screen still says it")
            s.view.destroy()
        }
    }

    // Scenario: A confirmation is gone after returning from the moderation
    // screen.
    // Requirement: a visit's definition names every main-area screen, so the
    // moderation screen ends a visit to the feed and returning from it begins
    // one, exactly as returning from a thread does.
    function test_a_confirmation_is_gone_after_returning_from_moderation() {
        var s = spec.openedOnStoaA()
        spec.publish(s.view, "post", "first post")
        compare(spec.outcomesShown(s.view, "post"), 1)

        spec.visibleNamed(s.view, "feed")[0].moderationRequested()
        compare(s.view.screenShown, "moderation")
        spec.visibleNamed(s.view, "moderationBackButton")[0].clicked()
        compare(s.view.screenShown, "feed")

        compare(spec.outcomesShown(s.view, "post"), 0,
                "returning from moderation begins a new visit to the feed")
        s.view.destroy()
    }

    // Scenario: Paging the feed does not withdraw the outcome.
    // Requirement: "Re-reading, paging or changing what a screen lists while it
    // stays rendered does not begin a new visit." Paging.
    function test_paging_the_feed_within_the_visit_keeps_the_outcome() {
        var s = spec.openedOnStoaA()
        s.bridge.hasMoreThreads = true
        spec.publish(s.view, "post", "first post")
        compare(spec.outcomesShown(s.view, "post"), 1)

        verify(spec.storedMessageShown(s.view, "post"),
               "it is the message for a newly stored op before any paging")

        spec.pressButtonReading(s.view, "Next")
        compare(spec.argsOfLast(s.bridge, "list_threads").page, 1,
                "the feed read the next page")
        compare(spec.outcomesShown(s.view, "post"), 1,
                "the outcome survives paging forward")
        verify(spec.storedMessageShown(s.view, "post"),
               "and it is still the message for a newly stored op, not another")

        spec.pressButtonReading(s.view, "Previous")
        compare(spec.argsOfLast(s.bridge, "list_threads").page, 0,
                "the feed read the first page again")
        compare(spec.outcomesShown(s.view, "post"), 1,
                "and paging back")
        verify(spec.storedMessageShown(s.view, "post"),
               "still the message for a newly stored op after paging back")
        s.view.destroy()
    }

    // Scenario: Changing what the feed lists does not withdraw the outcome.
    // The same rule, for changing what the screen lists.
    function test_changing_what_the_feed_lists_within_the_visit_keeps_the_outcome() {
        var s = spec.openedOnStoaA()
        spec.publish(s.view, "post", "first post")
        compare(spec.outcomesShown(s.view, "post"), 1)

        spec.toggleShowHidden(s.view)
        compare(spec.argsOfLast(s.bridge, "list_threads").includeHidden, true,
                "the feed re-read asking for hidden posts")
        compare(spec.outcomesShown(s.view, "post"), 1)
        verify(spec.storedMessageShown(s.view, "post"),
               "and it is still the message for a newly stored op")
        s.view.destroy()
    }

    // Scenario: Changing what the thread lists does not withdraw the outcome.
    function test_changing_what_the_thread_lists_within_the_visit_keeps_the_outcome() {
        var s = spec.openedOnStoaA()
        spec.openTheThread(s.view)
        spec.publish(s.view, "reply", "a reply")
        compare(spec.outcomesShown(s.view, "reply"), 1)

        spec.toggleShowHidden(s.view)
        compare(spec.argsOfLast(s.bridge, "read_thread").includeHidden, true,
                "the thread re-read asking for hidden posts")
        compare(spec.outcomesShown(s.view, "reply"), 1)
        verify(spec.storedMessageShown(s.view, "reply"),
               "and it is still the message for a newly stored op")
        s.view.destroy()
    }

    // Requirement: the outcome is displayed "until the visit ends or a later
    // publish from that composer reports its own outcome". Two publishes on ONE
    // visit, in each order, for each composer: the later outcome replaces the
    // earlier, so a stale confirmation does not stay over a refusal and a stale
    // refusal does not stay over a confirmation. (The scenario "A publish on the
    // later visit displays its own outcome" is a different clause: its second
    // publish is on a later visit.)
    function test_a_later_publish_on_the_same_visit_replaces_the_earlier_outcome() {
        var stored = { post: '{"opId":"newpost","wasNew":true}',
                       reply: '{"opId":"newreply","wasNew":true}' }
        var refused = '{"error":"the keystore is readable by others"}'
        var cases = [
            { kind: "post", first: stored.post, second: refused,
              firstSays: "Your post was saved on this machine.",
              secondSays: "Your post was not published." },
            { kind: "post", first: refused, second: stored.post,
              firstSays: "Your post was not published.",
              secondSays: "Your post was saved on this machine." },
            { kind: "reply", first: stored.reply, second: refused,
              firstSays: "Your reply was saved on this machine.",
              secondSays: "Your reply was not published." },
            { kind: "reply", first: refused, second: stored.reply,
              firstSays: "Your reply was not published.",
              secondSays: "Your reply was saved on this machine." }
        ]
        for (var i = 0; i < cases.length; i++) {
            var c = cases[i]
            var label = c.kind + " " + (i % 2 === 0 ? "stored then refused"
                                                    : "refused then stored")
            var s = spec.openedOnStoaA()
            if (c.kind === "reply")
                spec.openTheThread(s.view)
            s.bridge[c.kind + "Reply"] = c.first
            spec.publish(s.view, c.kind, "the first " + c.kind)
            verify(spec.visibleTextsContaining(s.view, c.firstSays).length > 0,
                   label + ": the first publish's outcome is displayed")

            s.bridge[c.kind + "Reply"] = c.second
            spec.publish(s.view, c.kind, "the second " + c.kind)

            verify(spec.visibleTextsContaining(s.view, c.secondSays).length > 0,
                   label + ": the later publish's outcome is displayed")
            compare(spec.visibleTextsContaining(s.view, c.firstSays), [],
                    label + ": and the earlier one is not")
            compare(spec.outcomesShown(s.view, c.kind), 1,
                    label + ": one outcome, not two")
            s.view.destroy()
        }
    }

    // Requirement: asking for hidden content "to be included or excluded"
    // changes what a screen lists, which does not begin a visit. The two
    // scenarios above press the toggle once, which only ever includes; this
    // presses it back, so excluding is observed too, on both composers.
    function test_asking_for_hidden_content_to_be_excluded_again_keeps_the_outcome() {
        var cases = [
            { kind: "post", method: "list_threads" },
            { kind: "reply", method: "read_thread" }
        ]
        for (var i = 0; i < cases.length; i++) {
            var c = cases[i]
            var s = spec.openedOnStoaA()
            if (c.kind === "reply")
                spec.openTheThread(s.view)
            spec.publish(s.view, c.kind, "a " + c.kind)
            verify(spec.storedMessageShown(s.view, c.kind),
                   c.kind + ": the message for a newly stored op is displayed")

            spec.toggleShowHidden(s.view)
            compare(spec.argsOfLast(s.bridge, c.method).includeHidden, true,
                    c.kind + ": hidden content was asked for")
            spec.toggleShowHidden(s.view)
            compare(spec.argsOfLast(s.bridge, c.method).includeHidden, false,
                    c.kind + ": and then asked to be excluded again")

            compare(spec.outcomesShown(s.view, c.kind), 1,
                    c.kind + ": the outcome survives the exclusion")
            verify(spec.storedMessageShown(s.view, c.kind),
                   c.kind + ": and it is still the message for a newly stored op")
            s.view.destroy()
        }
    }

    // Requirement: a composer on a later visit displays no outcome "after a
    // failed read is retried and succeeds on that visit". The thread's half is
    // `test_a_failed_read_on_the_later_visit_to_the_thread_carries_no_outcome`;
    // this is the feed's. Absence while the read is failed is not the case
    // under test (the composer is on screen then too, and the scenario above
    // pins it); absence once the retry succeeds is, because a screen that
    // merely hid an outcome while the read failed would show it again here.
    function test_a_retried_feed_read_on_the_later_visit_carries_no_outcome() {
        var s = spec.openedOnStoaA()
        spec.publish(s.view, "post", "first post")
        compare(spec.outcomesShown(s.view, "post"), 1)

        s.bridge.threadsFail = true
        spec.reopenFromTheList(s.view)
        compare(s.view.feedReadState, "failed", "the later visit's read failed")

        s.bridge.threadsFail = false
        spec.pressButtonReading(s.view, "Try reading again")
        compare(s.view.feedReadState, "ok", "the retry succeeded, on the same visit")

        compare(spec.visibleNamed(s.view, "postDraftField").length, 1,
                "the composer is on screen, so an outcome could be shown")
        compare(spec.outcomesShown(s.view, "post"), 0,
                "and it shows none: nothing was published on this visit")
        compare(spec.visibleTextsContaining(s.view, "saved on this machine"), [])
        s.view.destroy()
    }

    // ---- the draft, which this requirement does not decide -----------------
    //
    // Beginning a visit withdraws the outcome and leaves the draft alone. What
    // a draft belongs to is `composer-view`'s "A draft belongs to the target it
    // was entered for" and the requirements beside it, and
    // `tst_draft_targets.qml` holds their tests. The two below stay here because
    // they are the pair that pinned the old behaviour, one of which the spec
    // kept and one of which it reversed.

    // Scenario: A post draft is back when the same Stoa is reopened.
    function test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened() {
        var s = spec.openedOnStoaA()
        spec.visibleNamed(s.view, "postDraftField")[0].text = "half-written"

        spec.reopenFromTheList(s.view)

        compare(spec.visibleNamed(s.view, "postDraftField")[0].text, "half-written",
                "the draft typed before leaving is still in the field")
        s.view.destroy()
    }

    // Scenarios: "A draft typed in one Stoa is not in another Stoa's composer",
    // "Text typed in one Stoa is not published to another" and "A draft
    // returned to is published to its own Stoa".
    //
    // This test used to assert the opposite of its first two halves, as a pin
    // of what the view did while the spec was silent: one composer served every
    // Stoa, so the text followed the user into B and was published there.
    function test_a_draft_typed_in_one_stoa_is_neither_held_nor_published_in_another() {
        var s = spec.openedOnStoaA()
        spec.visibleNamed(s.view, "postDraftField")[0].text = "meant for A"

        spec.visibleNamed(s.view, "feedBackButton")[0].clicked()
        s.view.open(spec.stoaB, "Another Stoa", "")
        compare(s.view.chosen.stoa, spec.stoaB)

        compare(spec.visibleNamed(s.view, "postDraftField")[0].text, "",
                "B's composer is empty")
        var submits = spec.visibleNamed(s.view, "postSubmitButton")
        for (var i = 0; i < submits.length; i++)
            submits[i].clicked()
        compare(spec.countCalls(s.bridge, "publish_post"), 0,
                "and nothing B's feed offers publishes what was typed in A")

        spec.reopenFromTheList(s.view)
        compare(spec.visibleNamed(s.view, "postDraftField")[0].text, "meant for A",
                "reopening A shows the draft again")
        spec.visibleNamed(s.view, "postSubmitButton")[0].clicked()
        var sent = spec.argsOfLast(s.bridge, "publish_post")
        verify(sent !== null, "submitting it there reached publish_post")
        compare(sent.stoa, spec.stoaA, "naming A")
        compare(sent.body, "meant for A")
        s.view.destroy()
    }
}
