import QtQuick
import QtTest
import "../src/qml"

// `composer-view`: what an unsubmitted draft belongs to. Eight requirements,
// the one marked held by the last section below and the rest each with a
// section of their own:
//
//   - A draft belongs to the target it was entered for
//   - Two targets that differ never share a draft (the last section)
//   - Text is submitted only to the target it was entered for
//   - An unsubmitted draft is kept for its target while the view stays open
//   - A publish clears only the draft of the target it named
//   - A draft whose composer is not rendered stays held and is displayed nowhere
//   - An unsubmitted draft is held by the view alone and ends with it
//   - A restored draft is not announced
//
// **These tests drive `Main.qml`**, for the reason `tst_publish_outcome_visits`
// gives: the feed and the thread screen are each mounted once and re-pointed at
// whichever Stoa or thread the navigator chose, so one composer serves every
// target of its kind. That sharing is where text came to follow the user from
// one Stoa into another, and a test building a fresh composer per case cannot
// see it.
//
// **"Entered" is a write to the field's `text`**, which is what a keystroke
// does to it and what the end-to-end suite's typing does. Real key events need
// a window, and nothing asserted here depends on how the text arrived.
TestCase {
    id: spec
    name: "DraftTargets"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    Component { id: mainComponent; Main {} }
    Component { id: composerComponent; DComposer {} }

    readonly property string stoaA:
        "aaaaaaaa11111111111111111111111111111111111111111111111111111111"
    readonly property string stoaB:
        "bbbbbbbb22222222222222222222222222222222222222222222222222222222"
    readonly property string stoaC:
        "cccccccc33333333333333333333333333333333333333333333333333333333"
    readonly property string keyA:
        "k:0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"

    // Two thread roots. **Every Stoa of the fake lists both under the same op
    // ids**, which is what lets "two parents carrying the same op id in
    // different Stoas" be reached at all.
    readonly property string rootX: "cc" + "11".repeat(31)
    readonly property string rootY: "dd" + "22".repeat(31)

    readonly property string shutReason: "the keystore is readable by others"

    function rowFor(op, text) {
        return { thread: op, currentVersion: op, author: spec.keyA,
                 body: { text: text, removed: 0, marked: 0 },
                 attachments: [], isRevised: false, isHidden: false }
    }

    // A fake forum. It records every call, and what it answers depends on what
    // was asked:
    //
    //   - `get_capabilities` answers for the Stoa named: posting is possible
    //     unless that Stoa is in `shutStoas`.
    //   - `read_thread` answers with a root carrying the thread id it was asked
    //     for, or the error shape while `readThreadFail` is set.
    //   - `list_threads` answers the two roots, or the error shape while
    //     `threadsFail` is set.
    //   - `publish_post` and `publish_reply` answer `postReply` and
    //     `replyReply`, which a test sets to the outcome it wants.
    function forumBridge() {
        return {
            calls: [],
            shutStoas: ({}),
            threadsFail: false,
            readThreadFail: false,
            postReply: '{"opId":"newpost","wasNew":true}',
            replyReply: '{"opId":"newreply","wasNew":true}',
            callModule: function (module, method, args) {
                this.calls.push({ method: method, args: args })
                var asked = JSON.parse(String(args[0]))
                if (method === "list_stoas")
                    return JSON.stringify({ items: [
                        { stoa: spec.stoaA, foundingTitle: "Nym Research" }
                    ], page: 0, hasMore: false })
                if (method === "get_capabilities")
                    return this.shutStoas[asked.stoa] === true
                        ? JSON.stringify({ canPost: false, reason: spec.shutReason })
                        : '{"canPost":true,"reason":""}'
                if (method === "who_am_i")
                    return '{"hasIdentity":true,"publicKey":"' + spec.keyA
                           + '","recoveryNeedsTheRecord":false}'
                if (method === "list_threads") {
                    if (this.threadsFail)
                        return '{"error":"the store could not be read"}'
                    return JSON.stringify({ items: [
                        spec.rowFor(spec.rootX, "the first thread's root"),
                        spec.rowFor(spec.rootY, "the second thread's root")
                    ], page: 0, hasMore: false })
                }
                if (method === "read_thread") {
                    if (this.readThreadFail)
                        return '{"error":"that thread could not be read"}'
                    return JSON.stringify({ items: [{
                        thread: asked.thread, id: asked.thread,
                        currentVersion: asked.thread,
                        author: spec.keyA, isRevised: false,
                        moderation: { state: "unmoderated" }, position: "0",
                        body: { text: "a thread's root", removed: 0, marked: 0 }
                    }], page: 0, hasMore: false })
                }
                if (method === "publish_post")
                    return this.postReply
                if (method === "publish_reply")
                    return this.replyReply
                return '{"error":"no fake reply for ' + method + '"}'
            }
        }
    }

    // ---- what is displayed ------------------------------------------------
    //
    // The same definition `tst_publish_outcome_visits.qml` uses: a node counts
    // only where its own `visible` and every ancestor's are not false, so an
    // element inside a hidden screen or behind a shut gate is not displayed.
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

    // Every text on screen, in tree order. The draft field is one of them.
    function textsShown(view) {
        return spec.visibleNodes(view, function (node) {
            return typeof node.text === "string" && node.text !== ""
        }).map(function (node) {
            return node.text
        })
    }

    // Everything displayed, not only the texts: each displayed node's kind,
    // name, text, colour and border colour, in tree order. A marker that is a
    // dot or a tint rather than words shows up here and not in `textsShown`.
    //
    // The object's kind is read off its `String()` form with the address and the
    // per-instance counter removed, since those differ between two views that
    // display exactly the same thing.
    function displayedFingerprint(view) {
        return spec.visibleNodes(view, function (node) {
            return true
        }).map(function (node) {
            var kind = String(node).replace(/\(0x[0-9a-f]+(, "[^"]*")?\)/, "").replace(/_QML(TYPE)?_\d+/, "")
            var text = typeof node.text === "string" ? node.text : ""
            var color = node.color !== undefined ? String(node.color) : ""
            var border = node.border !== undefined
                ? String(node.border.color) + "/" + node.border.width : ""
            return [kind, node.objectName, text, color, border].join("|")
        })
    }

    function textsShownContaining(view, phrase) {
        return spec.textsShown(view).filter(function (text) {
            return text.indexOf(phrase) >= 0
        })
    }

    function outcomesShown(view, kind) {
        return spec.visibleNamed(view, kind + "OutcomeMessage").length
    }

    // ---- what the user does ------------------------------------------------

    // The one displayed draft field of `kind`. Fails where there is none, so a
    // test reading a field cannot pass against a composer that is not rendered.
    function field(view, kind) {
        var fields = spec.visibleNamed(view, kind + "DraftField")
        compare(fields.length, 1, "the " + kind + " composer is rendered")
        return fields[0]
    }

    function enter(view, kind, text) {
        spec.field(view, kind).text = text
    }

    function submit(view, kind) {
        var buttons = spec.visibleNamed(view, kind + "SubmitButton")
        compare(buttons.length, 1, "the " + kind + " composer offers its submit control")
        buttons[0].clicked()
    }

    // Every submit affordance the displayed screen offers, of either kind.
    // Returns how many there were.
    function pressEverySubmit(view) {
        var buttons = spec.visibleNodes(view, function (node) {
            return node.objectName === "postSubmitButton"
                || node.objectName === "replySubmitButton"
        })
        for (var i = 0; i < buttons.length; i++)
            buttons[i].clicked()
        return buttons.length
    }

    function pressButtonReading(view, text) {
        var buttons = spec.visibleNodes(view, function (node) {
            return node.text === text && typeof node.clicked === "function"
        })
        compare(buttons.length, 1, "exactly one displayed control reads '" + text + "'")
        buttons[0].clicked()
    }

    // Back to the Stoa list through the controls on screen, from wherever the
    // view is.
    function leaveToTheList(view) {
        if (view.screenShown === "thread")
            spec.backToTheFeed(view)
        if (view.screenShown === "feed")
            spec.visibleNamed(view, "feedBackButton")[0].clicked()
        compare(view.screenShown, "list")
    }

    // Open a Stoa from the list. Through `open()`, which is what the list's
    // `stoaChosen` handler calls: the fake lists one Stoa, and the others are
    // reached by the navigator's own transition, as
    // `tst_publish_outcome_visits.qml` reaches its second one.
    function openStoa(view, stoa) {
        compare(view.screenShown, "list", "a Stoa is opened from the list")
        view.open(stoa, "A Stoa", "")
        compare(view.screenShown, "feed")
        compare(view.chosen.stoa, stoa)
    }

    // Open a thread from the feed on screen, by raising the feed's
    // `threadOpened` as `tst_navigation.qml` does.
    function openThread(view, root) {
        compare(view.screenShown, "feed", "a thread is opened from a feed")
        spec.visibleNamed(view, "feed")[0].threadOpened(root)
        compare(view.screenShown, "thread")
        compare(view.reading.rootOp, root)
    }

    function backToTheFeed(view) {
        spec.visibleNamed(view, "threadBackButton")[0].clicked()
        compare(view.screenShown, "feed")
    }

    // From the list to a Stoa's feed, or on to one of its threads.
    function goTo(view, stoa, root) {
        spec.leaveToTheList(view)
        spec.openStoa(view, stoa)
        if (root !== undefined)
            spec.openThread(view, root)
    }

    function started() {
        var bridge = spec.forumBridge()
        Core.bridge = bridge
        var view = mainComponent.createObject(null, {})
        compare(view.screenShown, "list")
        return { view: view, bridge: bridge }
    }

    // ---- what was published -------------------------------------------------

    // Every publish call made, in order, as what it named and what it carried.
    function publishes(bridge) {
        var made = []
        for (var i = 0; i < bridge.calls.length; i++) {
            var method = bridge.calls[i].method
            if (method !== "publish_post" && method !== "publish_reply")
                continue
            var sent = JSON.parse(String(bridge.calls[i].args[0]))
            made.push({ method: method, stoa: sent.stoa,
                        parent: sent.parent === undefined ? "" : sent.parent,
                        body: sent.body })
        }
        return made
    }

    function publishesCarrying(bridge, body) {
        return spec.publishes(bridge).filter(function (made) {
            return made.body === body
        })
    }

    // =====================================================================
    // Requirement: A draft belongs to the target it was entered for
    // =====================================================================

    // Scenario: A draft typed in one Stoa is not in another Stoa's composer.
    function test_a_draft_typed_in_one_stoa_is_not_in_another_stoas_composer() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "meant for A")

        spec.goTo(s.view, spec.stoaB)

        compare(spec.field(s.view, "post").text, "")
        s.view.destroy()
    }

    // Scenario: A reply typed in one thread is not in another thread of the
    // same Stoa.
    function test_a_reply_typed_in_one_thread_is_not_in_another_thread_of_the_same_stoa() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "answering X")

        spec.backToTheFeed(s.view)
        spec.openThread(s.view, spec.rootY)

        compare(spec.field(s.view, "reply").text, "")
        s.view.destroy()
    }

    // Scenario: A reply typed in one thread is not in a thread of a different
    // Stoa.
    function test_a_reply_typed_in_one_thread_is_not_in_a_thread_of_a_different_stoa() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "answering X in A")

        spec.goTo(s.view, spec.stoaB, spec.rootY)

        compare(spec.field(s.view, "reply").text, "")
        s.view.destroy()
    }

    // Scenario: Two parents carrying the same op id in different Stoas do not
    // share a draft. A draft keyed by its parent alone passes the scenario
    // above, where the op ids differ, and fails this one.
    function test_two_parents_with_one_op_id_in_different_stoas_do_not_share_a_draft() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "answering X in A")

        spec.goTo(s.view, spec.stoaB, spec.rootX)

        compare(s.view.reading.stoa, spec.stoaB)
        compare(s.view.reading.rootOp, spec.rootX, "the same op id, in another Stoa")
        compare(spec.field(s.view, "reply").text, "")
        s.view.destroy()
    }

    // Scenarios: "A post draft and a reply draft in one Stoa are separate" and
    // "A reply draft is not in the post composer of its Stoa".
    function test_a_post_draft_and_a_reply_draft_in_one_stoa_are_separate() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "a post for A")

        spec.openThread(s.view, spec.rootX)
        compare(spec.field(s.view, "reply").text, "",
                "the Stoa's post draft is not in its thread's reply composer")
        s.view.destroy()

        s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "a reply to X")

        spec.backToTheFeed(s.view)
        compare(spec.field(s.view, "post").text, "",
                "a thread's reply draft is not in its Stoa's post composer")
        s.view.destroy()
    }

    // The field's own undo is an affordance on it like any other, so text
    // entered for another target must not be reachable through it either.
    //
    // Entered with `insert()` here and not by writing `text`: a write to `text`
    // resets the undo history itself, so a field filled that way has nothing to
    // undo and the test would pass whatever the composer did. The `canUndo`
    // assertion is what shows there was an edit to step back to.
    function test_undo_in_another_stoas_field_does_not_bring_the_first_stoas_text() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.field(s.view, "post").insert(0, "meant for A")
        compare(spec.field(s.view, "post").text, "meant for A")
        compare(spec.field(s.view, "post").canUndo, true,
                "in A, the edit can be undone")

        spec.goTo(s.view, spec.stoaB)
        var field = spec.field(s.view, "post")
        field.undo()
        compare(field.text, "", "undo in B brings nothing back")
        field.redo()
        compare(field.text, "", "and neither does redo")

        spec.goTo(s.view, spec.stoaA)
        compare(spec.field(s.view, "post").text, "meant for A",
                "and A's draft is still A's")
        s.view.destroy()
    }

    // =====================================================================
    // Requirement: Text is submitted only to the target it was entered for
    //
    // Judged on the publish calls that went out, never on what a field shows.
    // =====================================================================

    // Scenario: Text typed in one Stoa is not published to another.
    function test_text_typed_in_one_stoa_is_not_published_to_another() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "meant for A")

        spec.goTo(s.view, spec.stoaB)
        spec.pressEverySubmit(s.view)

        compare(spec.publishesCarrying(s.bridge, "meant for A").length, 0)
        s.view.destroy()
    }

    // Scenario: A post written in the second Stoa is published there with its
    // own text.
    function test_a_post_written_in_the_second_stoa_is_published_there_with_its_own_text() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "meant for A")

        spec.goTo(s.view, spec.stoaB)
        spec.enter(s.view, "post", "meant for B")
        spec.submit(s.view, "post")

        var made = spec.publishes(s.bridge)
        compare(made.length, 1, "exactly one publish call")
        compare(made[0].method, "publish_post")
        compare(made[0].stoa, spec.stoaB)
        compare(made[0].body, "meant for B")
        s.view.destroy()
    }

    // Scenario: A draft returned to is published to its own Stoa.
    function test_a_draft_returned_to_is_published_to_its_own_stoa() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "meant for A")

        spec.goTo(s.view, spec.stoaB)
        spec.goTo(s.view, spec.stoaA)
        spec.submit(s.view, "post")

        var made = spec.publishes(s.bridge)
        compare(made.length, 1)
        compare(made[0].stoa, spec.stoaA)
        compare(made[0].body, "meant for A")
        s.view.destroy()
    }

    // Scenarios: "A reply typed in one thread is not published to another
    // thread of the same Stoa" and "… to a thread of a different Stoa". The
    // third case is the same op id in another Stoa, which the first two do not
    // reach.
    function test_a_reply_typed_in_one_thread_is_not_published_to_another() {
        var cases = [
            { label: "another thread of the same Stoa", stoa: spec.stoaA, root: spec.rootY },
            { label: "a thread of a different Stoa", stoa: spec.stoaB, root: spec.rootY },
            { label: "the same op id in a different Stoa", stoa: spec.stoaB, root: spec.rootX }
        ]
        for (var i = 0; i < cases.length; i++) {
            var s = spec.started()
            spec.goTo(s.view, spec.stoaA, spec.rootX)
            spec.enter(s.view, "reply", "answering X in A")

            spec.goTo(s.view, cases[i].stoa, cases[i].root)
            spec.pressEverySubmit(s.view)

            compare(spec.publishesCarrying(s.bridge, "answering X in A").length, 0,
                    cases[i].label)
            s.view.destroy()
        }
    }

    // Scenario: A reply written in the second thread names that thread's
    // parent and Stoa.
    function test_a_reply_written_in_the_second_thread_names_that_threads_parent_and_stoa() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "answering X in A")

        spec.goTo(s.view, spec.stoaB, spec.rootY)
        spec.enter(s.view, "reply", "answering Y in B")
        spec.submit(s.view, "reply")

        var made = spec.publishes(s.bridge)
        compare(made.length, 1, "exactly one publish call")
        compare(made[0].method, "publish_reply")
        compare(made[0].stoa, spec.stoaB)
        compare(made[0].parent, spec.rootY)
        compare(made[0].body, "answering Y in B")
        s.view.destroy()
    }

    // Scenario: A reply draft returned to is published to its own parent.
    function test_a_reply_draft_returned_to_is_published_to_its_own_parent() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "answering X in A")

        spec.backToTheFeed(s.view)
        spec.openThread(s.view, spec.rootY)
        spec.backToTheFeed(s.view)
        spec.openThread(s.view, spec.rootX)
        spec.submit(s.view, "reply")

        var made = spec.publishes(s.bridge)
        compare(made.length, 1)
        compare(made[0].stoa, spec.stoaA)
        compare(made[0].parent, spec.rootX)
        compare(made[0].body, "answering X in A")
        s.view.destroy()
    }

    // =====================================================================
    // Requirement: An unsubmitted draft is kept for its target while the view
    // stays open
    //
    // "A post draft is back when the same Stoa is reopened" is
    // `tst_publish_outcome_visits.qml`'s
    // `test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened`.
    // =====================================================================

    // Scenario: A post draft is back after a thread was opened from the feed.
    function test_a_post_draft_is_back_after_a_thread_was_opened_from_the_feed() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "half-written")

        spec.openThread(s.view, spec.rootX)
        spec.backToTheFeed(s.view)

        compare(spec.field(s.view, "post").text, "half-written")
        s.view.destroy()
    }

    // Scenario: A reply draft is back when the same thread is reopened.
    function test_a_reply_draft_is_back_when_the_same_thread_is_reopened() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "half a reply")

        spec.backToTheFeed(s.view)
        spec.openThread(s.view, spec.rootX)

        compare(spec.field(s.view, "reply").text, "half a reply")
        s.view.destroy()
    }

    // Scenario: A draft is back after other targets were visited and written
    // in.
    function test_a_draft_is_back_after_other_targets_were_visited_and_written_in() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "meant for A")
        spec.goTo(s.view, spec.stoaB)
        spec.enter(s.view, "post", "meant for B")

        spec.goTo(s.view, spec.stoaA)
        compare(spec.field(s.view, "post").text, "meant for A")
        spec.goTo(s.view, spec.stoaB)
        compare(spec.field(s.view, "post").text, "meant for B")
        s.view.destroy()
    }

    // Scenario: The draft held is the text as last edited.
    function test_the_draft_held_is_the_text_as_last_edited() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "first wording")

        spec.goTo(s.view, spec.stoaA)
        compare(spec.field(s.view, "post").text, "first wording")
        spec.enter(s.view, "post", "second wording")

        spec.goTo(s.view, spec.stoaA)
        compare(spec.field(s.view, "post").text, "second wording")
        s.view.destroy()
    }

    // Scenario: A draft the user emptied is not brought back.
    function test_a_draft_the_user_emptied_is_not_brought_back() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "written and then regretted")
        spec.enter(s.view, "post", "")

        spec.goTo(s.view, spec.stoaA)

        compare(spec.field(s.view, "post").text, "")
        s.view.destroy()
    }

    // Scenario: A draft comes back with every character it had. For each kind
    // of composer, and with another target visited in between so the text was
    // really put back and not merely left in the field.
    function test_a_draft_comes_back_with_every_character_it_had() {
        // Leading and trailing whitespace, CJK, an astral code point, a bidi
        // override and its pop, a zero-width space and joiner, a BOM, a tab and
        // newlines.
        //
        // **Spelled as `\u{...}` escapes, not typed**: the invisible ones cannot be seen
        // in an editor or a diff, and a formatter or a paste that stripped them
        // would leave a test that passes over a plainer string. The length is
        // pinned below for the same reason: 29 code units, counted by hand
        // from the pieces on this line.
        var typed = "  \t中文 🏛 \u{202E}abc\u{202C} x\u{200B}y\u{200D}z\u{FEFF}\n\nend \n "
        compare(typed.length, 29, "the text under test is the one described above")
        verify(typed.indexOf("\u{202E}") >= 0 && typed.indexOf("\u{202C}") >= 0
               && typed.indexOf("\u{200B}") >= 0 && typed.indexOf("\u{200D}") >= 0
               && typed.indexOf("\u{FEFF}") >= 0, "and it holds each invisible character")
        var cases = [
            { kind: "post", root: undefined, elsewhere: spec.stoaB },
            { kind: "reply", root: spec.rootX, elsewhere: spec.stoaB }
        ]
        for (var i = 0; i < cases.length; i++) {
            var c = cases[i]
            var s = spec.started()
            spec.goTo(s.view, spec.stoaA, c.root)
            spec.enter(s.view, c.kind, typed)

            spec.goTo(s.view, c.elsewhere, c.root)
            compare(spec.field(s.view, c.kind).text, "", c.kind + ": not there meanwhile")
            spec.goTo(s.view, spec.stoaA, c.root)

            var back = spec.field(s.view, c.kind).text
            compare(back.length, typed.length, c.kind + ": no character added or dropped")
            verify(back === typed, c.kind + ": and each one is the one entered")
            s.view.destroy()
        }
    }

    // Scenario: Drafts for several targets are all held at once.
    function test_drafts_for_several_targets_are_all_held_at_once() {
        var targets = [
            { kind: "post", stoa: spec.stoaA, root: undefined, text: "post for A" },
            { kind: "post", stoa: spec.stoaB, root: undefined, text: "post for B" },
            { kind: "post", stoa: spec.stoaC, root: undefined, text: "post for C" },
            { kind: "reply", stoa: spec.stoaA, root: spec.rootX, text: "reply to X in A" },
            { kind: "reply", stoa: spec.stoaA, root: spec.rootY, text: "reply to Y in A" }
        ]
        var s = spec.started()
        for (var i = 0; i < targets.length; i++) {
            spec.goTo(s.view, targets[i].stoa, targets[i].root)
            spec.enter(s.view, targets[i].kind, targets[i].text)
        }

        for (var j = 0; j < targets.length; j++) {
            spec.goTo(s.view, targets[j].stoa, targets[j].root)
            compare(spec.field(s.view, targets[j].kind).text, targets[j].text)
        }
        s.view.destroy()
    }

    // Scenarios: "A draft kept by a refusal is back without the refusal" and
    // "A draft kept by an already-published outcome is back without that
    // outcome".
    function test_a_draft_kept_by_an_outcome_is_back_without_that_outcome() {
        var cases = [
            { label: "refused", reply: '{"error":"the keystore is readable by others"}',
              says: "was not published" },
            { label: "already published", reply: '{"opId":"newpost","wasNew":false}',
              says: "already published" }
        ]
        for (var i = 0; i < cases.length; i++) {
            var s = spec.started()
            s.bridge.postReply = cases[i].reply
            spec.goTo(s.view, spec.stoaA)
            spec.enter(s.view, "post", "what was submitted")
            spec.submit(s.view, "post")
            verify(spec.textsShownContaining(s.view, cases[i].says).length > 0,
                   cases[i].label + ": the outcome is displayed on the visit that produced it")

            spec.goTo(s.view, spec.stoaA)

            compare(spec.field(s.view, "post").text, "what was submitted", cases[i].label)
            compare(spec.outcomesShown(s.view, "post"), 0, cases[i].label)
            s.view.destroy()
        }
    }

    // =====================================================================
    // Requirement: A publish clears only the draft of the target it named
    // =====================================================================

    // Scenario: A published draft is not brought back.
    function test_a_published_draft_is_not_brought_back() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "a finished post")
        spec.submit(s.view, "post")
        compare(spec.publishesCarrying(s.bridge, "a finished post").length, 1)

        spec.goTo(s.view, spec.stoaA)

        compare(spec.field(s.view, "post").text, "")
        s.view.destroy()
    }

    // Scenario: A publish in one Stoa leaves another Stoa's draft held.
    function test_a_publish_in_one_stoa_leaves_another_stoas_draft_held() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "meant for A")

        spec.goTo(s.view, spec.stoaB)
        spec.enter(s.view, "post", "meant for B")
        spec.submit(s.view, "post")
        compare(spec.field(s.view, "post").text, "", "B's draft was cleared by its publish")

        spec.goTo(s.view, spec.stoaA)
        compare(spec.field(s.view, "post").text, "meant for A")
        s.view.destroy()
    }

    // Scenario: A published reply leaves the Stoa's post draft held.
    function test_a_published_reply_leaves_the_stoas_post_draft_held() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "a post for A")

        spec.openThread(s.view, spec.rootX)
        spec.enter(s.view, "reply", "a reply to X")
        spec.submit(s.view, "reply")
        compare(spec.field(s.view, "reply").text, "", "the reply was cleared by its publish")

        spec.backToTheFeed(s.view)
        compare(spec.field(s.view, "post").text, "a post for A")
        s.view.destroy()
    }

    // Scenario: A published reply leaves another thread's reply draft held.
    function test_a_published_reply_leaves_another_threads_reply_draft_held() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "a reply to X")

        spec.backToTheFeed(s.view)
        spec.openThread(s.view, spec.rootY)
        spec.enter(s.view, "reply", "a reply to Y")
        spec.submit(s.view, "reply")
        compare(spec.field(s.view, "reply").text, "", "Y's reply was cleared by its publish")

        spec.backToTheFeed(s.view)
        spec.openThread(s.view, spec.rootX)
        compare(spec.field(s.view, "reply").text, "a reply to X")
        s.view.destroy()
    }

    // Scenario: A publish that reports after its composer was pointed elsewhere
    // clears the target it named.
    // Requirement: "the draft cleared MUST be the one held for the target that
    // publish named, and a draft held for any other target MUST be left as it
    // was."
    //
    // **On one composer, with the composer re-pointed while the publish is
    // outstanding.** No screen can do that today: the call into core is
    // synchronous, so nothing navigates between a submit and its answer. The
    // fake does it from inside the call, which is the only place it can be
    // done, and it is what tells "clear the target the publish named" from
    // "clear whatever the field holds when the answer arrives". The two agree
    // in every test above.
    function test_a_publish_answered_after_the_composer_was_re_pointed_clears_only_what_it_named() {
        var composer = composerComponent.createObject(null, {
            kind: "post", stoaAddress: spec.stoaB })
        composer.draft = "meant for B"
        composer.stoaAddress = spec.stoaA
        composer.draft = "meant for A"

        var sent = []
        Core.bridge = {
            callModule: function (module, method, args) {
                sent.push(JSON.parse(String(args[0])))
                composer.stoaAddress = spec.stoaB
                return '{"opId":"newpost","wasNew":true}'
            }
        }
        composer.submit()

        compare(sent.length, 1)
        compare(sent[0].stoa, spec.stoaA)
        compare(sent[0].body, "meant for A")
        compare(composer.draft, "meant for B",
                "the draft of the Stoa now shown was not the one published, and is kept")
        composer.stoaAddress = spec.stoaA
        compare(composer.draft, "", "the draft of the Stoa the publish named is cleared")
        composer.destroy()
    }

    // =====================================================================
    // Requirement: A draft whose composer is not rendered stays held and is
    // displayed nowhere
    // =====================================================================

    // Scenarios: "A draft behind a shut gate is not displayed" and "A draft is
    // back when the gate opens again".
    function test_a_post_draft_behind_a_shut_gate_is_held_unseen_and_comes_back() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "written while posting was possible")

        s.bridge.shutStoas[spec.stoaA] = true
        spec.goTo(s.view, spec.stoaA)
        verify(spec.textsShownContaining(s.view, spec.shutReason).length > 0,
               "the gate is shut on this visit")
        compare(spec.visibleNamed(s.view, "postDraftField").length, 0,
                "no text input for composing is rendered")
        compare(spec.textsShownContaining(s.view, "written while posting was possible"), [],
                "and the draft is displayed nowhere")

        s.bridge.shutStoas[spec.stoaA] = false
        spec.goTo(s.view, spec.stoaA)
        compare(spec.field(s.view, "post").text, "written while posting was possible")
        s.view.destroy()
    }

    // Scenario: A reply draft behind a shut gate is held and comes back.
    function test_a_reply_draft_behind_a_shut_gate_is_held_unseen_and_comes_back() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.enter(s.view, "reply", "a reply written while posting was possible")

        s.bridge.shutStoas[spec.stoaA] = true
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        verify(spec.textsShownContaining(s.view, spec.shutReason).length > 0,
               "the gate is shut on this visit")
        compare(spec.visibleNamed(s.view, "replyDraftField").length, 0,
                "no text input for a reply is rendered")
        compare(spec.textsShownContaining(s.view, "a reply written while posting was possible"),
                [], "and the draft is displayed nowhere")

        s.bridge.shutStoas[spec.stoaA] = false
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        compare(spec.field(s.view, "reply").text, "a reply written while posting was possible")
        s.view.destroy()
    }

    // The gate is per Stoa, and so is what it hides: a Stoa whose gate is open
    // does not show the draft a shut one is holding.
    function test_a_draft_behind_one_stoas_shut_gate_is_not_in_an_open_stoas_composer() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "meant for A")

        s.bridge.shutStoas[spec.stoaA] = true
        spec.goTo(s.view, spec.stoaA)
        compare(spec.visibleNamed(s.view, "postDraftField").length, 0)
        spec.goTo(s.view, spec.stoaB)

        compare(spec.field(s.view, "post").text, "")
        compare(spec.textsShownContaining(s.view, "meant for A"), [])
        s.view.destroy()
    }

    // Scenarios: A draft is back when a failed read recovers (the post case),
    // and A reply draft is back when a failed thread read recovers (the reply
    // case); the table below drives both. And the clause of the requirement
    // they sit under, for the failed state itself: the draft is displayed
    // nowhere but in the field of a composer that is rendered.
    //
    // **Whether a composer is rendered while a read has failed is the screen's
    // own business and is not asserted.** The thread screen stops rendering
    // its reply composer; the feed's post composer is gated on the posting
    // probe alone and stays rendered through a failed list read. The
    // requirement binds the text, not the composer: so what is checked is
    // which nodes carry the draft, and every displayed one must be that
    // kind's draft field, by name. A banner, a label or a second copy anywhere
    // else is a node of another name and fails this; so does text still shown
    // where no field is rendered.
    function test_a_draft_is_back_when_a_failed_read_recovers() {
        var draft = "written before the read failed"
        var cases = [
            { kind: "post", root: undefined, flag: "threadsFail" },
            { kind: "reply", root: spec.rootX, flag: "readThreadFail" }
        ]
        for (var i = 0; i < cases.length; i++) {
            var c = cases[i]
            var s = spec.started()
            spec.goTo(s.view, spec.stoaA, c.root)
            spec.enter(s.view, c.kind, draft)

            spec.leaveToTheList(s.view)
            s.bridge[c.flag] = true
            spec.openStoa(s.view, spec.stoaA)
            if (c.kind === "post") {
                compare(s.view.feedReadState, "failed", "the feed read failed")
            } else {
                spec.openThread(s.view, c.root)
                compare(s.view.threadReadState, "failed", "the thread read failed")
            }

            // Which nodes, and not how many: every displayed node carrying the
            // draft is a composer's draft field. A count alone is green over a
            // field that lost its text beside a banner that gained it.
            var fieldName = c.kind + "DraftField"
            var carrying = spec.visibleNodes(s.view, function (node) {
                return typeof node.text === "string" && node.text.indexOf(draft) >= 0
            })
            for (var n = 0; n < carrying.length; n++)
                compare(carrying[n].objectName, fieldName,
                        c.kind + ": while the read is failed, the draft is displayed "
                        + "only in a composer's draft field, but node " + n + " is "
                        + String(carrying[n]))

            s.bridge[c.flag] = false
            spec.pressButtonReading(s.view, "Try reading again")

            compare(spec.field(s.view, c.kind).text, draft, c.kind)
            s.view.destroy()
        }
    }

    // =====================================================================
    // Requirement: An unsubmitted draft is held by the view alone and ends
    // with it
    // =====================================================================

    // Scenario: An unsubmitted draft reaches no core call.
    function test_an_unsubmitted_draft_reaches_no_core_call() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "POST-DRAFT-MARKER")
        spec.openThread(s.view, spec.rootX)
        spec.enter(s.view, "reply", "REPLY-DRAFT-MARKER")

        spec.backToTheFeed(s.view)
        spec.goTo(s.view, spec.stoaB, spec.rootY)
        spec.goTo(s.view, spec.stoaA, spec.rootX)
        spec.leaveToTheList(s.view)

        verify(s.bridge.calls.length > 0, "the view did call core while moving about")
        for (var i = 0; i < s.bridge.calls.length; i++) {
            var raw = s.bridge.calls[i].method + " " + JSON.stringify(s.bridge.calls[i].args)
            verify(raw.indexOf("POST-DRAFT-MARKER") < 0, raw)
            verify(raw.indexOf("REPLY-DRAFT-MARKER") < 0, raw)
        }
        s.view.destroy()
    }

    // Scenario: A view opened anew holds no draft.
    function test_a_view_opened_anew_holds_no_draft() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "written in the first view")
        spec.openThread(s.view, spec.rootX)
        spec.enter(s.view, "reply", "a reply written in the first view")
        s.view.destroy()

        var anew = mainComponent.createObject(null, {})
        spec.goTo(anew, spec.stoaA)
        compare(spec.field(anew, "post").text, "")
        spec.openThread(anew, spec.rootX)
        compare(spec.field(anew, "reply").text, "")
        anew.destroy()
    }

    // =====================================================================
    // Requirement: A restored draft is not announced
    // =====================================================================

    // Scenario: A restored draft looks like the same text typed afresh.
    //
    // Everything the screen displays is compared, the field included, so a
    // marker anywhere on the feed would differ. Another Stoa is visited in
    // between: the text on the second visit to A was put back by the view.
    function test_a_restored_draft_looks_like_the_same_text_typed_afresh() {
        var cases = [
            { kind: "post", root: undefined },
            { kind: "reply", root: spec.rootX }
        ]
        for (var i = 0; i < cases.length; i++) {
            var c = cases[i]
            var restored = spec.started()
            spec.goTo(restored.view, spec.stoaA, c.root)
            spec.enter(restored.view, c.kind, "the same words")
            spec.goTo(restored.view, spec.stoaB, c.root)
            spec.goTo(restored.view, spec.stoaA, c.root)
            compare(spec.field(restored.view, c.kind).text, "the same words")
            var whenRestored = spec.textsShown(restored.view)
            var shapeWhenRestored = spec.displayedFingerprint(restored.view)
            restored.view.destroy()

            var fresh = spec.started()
            spec.goTo(fresh.view, spec.stoaA, c.root)
            spec.enter(fresh.view, c.kind, "the same words")
            var whenTyped = spec.textsShown(fresh.view)
            var shapeWhenTyped = spec.displayedFingerprint(fresh.view)
            fresh.view.destroy()

            verify(whenTyped.indexOf("the same words") >= 0,
                   c.kind + ": the comparison includes the field")
            compare(whenRestored, whenTyped, c.kind)
            // A marker that is not words: a dot, a tint, a border.
            verify(shapeWhenTyped.length > 20,
                   c.kind + ": the fingerprint covers the screen and not a stub")
            compare(shapeWhenRestored, shapeWhenTyped, c.kind + ": nothing else differs either")
        }
    }

    // Scenario: A restored draft carries the same warning as when it was
    // typed.
    function test_a_restored_draft_carries_the_same_warning_as_when_it_was_typed() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        // A right-to-left override and a zero-width space: the two the
        // warning counts, spelled as `\u{...}` escapes because neither can be seen.
        spec.enter(s.view, "post", "safe\u{202E}text\u{200B}more")
        verify(spec.textsShownContaining(s.view, "contains 2 invisible character(s)").length > 0,
               "the warning names how many were found when the text was typed")
        var before = spec.textsShown(s.view)

        spec.goTo(s.view, spec.stoaB)
        compare(spec.textsShownContaining(s.view, "invisible character(s)"), [],
                "another Stoa's empty composer carries no such warning")
        spec.goTo(s.view, spec.stoaA)

        verify(spec.textsShownContaining(s.view, "contains 2 invisible character(s)").length > 0)
        compare(spec.textsShown(s.view), before,
                "and nothing is displayed that was not displayed before the user left")
        s.view.destroy()
    }

    // Scenario: A restored over-length draft is still withheld.
    function test_a_restored_over_length_draft_is_still_withheld() {
        // One byte past the 150 KiB cap the feed's composer applies.
        var typed = "a".repeat(150 * 1024 + 1)
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", typed)
        verify(spec.textsShownContaining(s.view, "longer than the core module will accept").length > 0,
               "over the limit when typed")

        spec.goTo(s.view, spec.stoaB)
        compare(spec.visibleNamed(s.view, "postSubmitButton").length, 0)
        compare(spec.textsShownContaining(s.view, "longer than the core module will accept"), [])
        spec.goTo(s.view, spec.stoaA)

        var back = spec.field(s.view, "post").text
        compare(back.length, typed.length, "the full text, not a truncated one")
        verify(back === typed)
        verify(spec.textsShownContaining(s.view, "longer than the core module will accept").length > 0,
               "the view reports that it is too long")
        compare(spec.visibleNamed(s.view, "postSubmitButton").length, 0,
                "and the submit affordance is unavailable")
        compare(spec.publishes(s.bridge).length, 0)
        s.view.destroy()
    }

    // =====================================================================
    // Scenarios that need a composer driven directly, or a screen the others
    // do not visit; the requirement text each rests on is quoted. A test below
    // that quotes only a requirement, and no scenario, pins what the
    // requirement says and the scenario beside it does not.
    // =====================================================================

    // Requirement: *Two targets that differ never share a draft*, whatever
    // characters the Stoa address or the parent op contain.
    // Scenario: Targets whose address and parent read alike when joined do not
    // share a draft.
    //
    // On a composer directly: a Stoa address is 64 hex characters in this view,
    // so no pair like this can be reached through `Main.qml`, and the key must
    // not depend on that.
    //
    // **The separator is every UTF-16 code unit, 0 to 0xFFFF, and none.** The
    // scenario says "any other single character, or none", and a hand-written
    // list of them holds only for the ones its author thought of: a key joined
    // with a character outside the list passed an earlier version of this
    // test. A key joining the parts with any one character `s` maps
    // `("a" + s + "b", "c")` and `("a", "b" + s + "c")` onto one string, so the
    // sweep reaches whichever `s` a mutant picks. The pair is built from `s`
    // and not from a table of known-colliding rows, so it needs no maintenance.
    //
    // **And a sample of characters outside the BMP**, each two code units
    // long, since "any other single character" includes them and a code-unit
    // sweep builds no pair from one. 0x10FFFF cannot be swept: a million
    // iterations, and the sweep is most of this file's runtime already. So
    // these four are a sample (the first and last code points outside the BMP,
    // the Stoa-like U+1F3DB, and U+1F600), and a join on an astral character
    // outside the sample passes.
    //
    // One composer serves the sweep: each pair differs from every other pair
    // in `s`, and a composer per code unit would only be slower.
    function test_two_targets_whose_parts_run_together_alike_do_not_share_a_draft() {
        var composer = composerComponent.createObject(null, { kind: "reply" })
        var separators = [{ sep: "", which: "no separator" }]
        for (var unit = 0; unit <= 0xFFFF; unit++)
            separators.push({ sep: String.fromCharCode(unit),
                              which: "separator U+" + unit.toString(16) })
        separators.push({ sep: "\u{10000}", which: "separator U+10000" })
        separators.push({ sep: "\u{1F3DB}", which: "separator U+1f3db" })
        separators.push({ sep: "\u{1F600}", which: "separator U+1f600" })
        separators.push({ sep: "\u{10FFFF}", which: "separator U+10ffff" })
        for (var k = 0; k < separators.length; k++) {
            var sep = separators[k].sep
            var which = separators[k].which
            var first = "for the first, " + which

            composer.stoaAddress = "a" + sep + "b"
            composer.parentOp = "c"
            composer.draft = first

            composer.stoaAddress = "a"
            composer.parentOp = "b" + sep + "c"
            if (composer.draft !== "")
                fail(which + ": the second target holds '" + composer.draft + "'")

            composer.stoaAddress = "a" + sep + "b"
            composer.parentOp = "c"
            if (composer.draft !== first)
                fail(which + ": the first target no longer holds its own, but '"
                     + composer.draft + "'")
        }
        composer.destroy()
    }

    // Requirement: *Two targets that differ never share a draft*: "Two targets
    // are the same target only where both are for a post or both for a reply".
    // A reply to the empty parent and a post name the same Stoa address and the
    // same parent text, and are still two targets.
    function test_a_post_and_a_reply_to_the_empty_parent_do_not_share_a_draft() {
        var composer = composerComponent.createObject(null, {
            kind: "post", stoaAddress: spec.stoaA, parentOp: "" })
        composer.draft = "meant as a post"

        composer.kind = "reply"
        compare(composer.draft, "", "the same address and parent, as a reply")
        composer.draft = "meant as a reply"

        composer.kind = "post"
        compare(composer.draft, "meant as a post", "and the post is still its own")
        composer.kind = "reply"
        compare(composer.draft, "meant as a reply", "and so is the reply")
        composer.destroy()
    }

    // Requirement: *A draft belongs to the target it was entered for*: a post's
    // target is "the Stoa address", and nothing else. A post composer holds no
    // reply parent for a publish to name, so a `parentOp` it is handed (no
    // screen hands one today) neither moves its draft nor reaches the publish.
    //
    // `design.md`, Decision 2: keying a post on `parentOp` in place of
    // `replyParent` is invisible wherever `Main.qml` is driven, since no screen
    // gives a post composer a parent. This is the test that pins it.
    function test_a_posts_draft_does_not_move_with_a_parent_it_is_never_sent() {
        var composer = composerComponent.createObject(null, {
            kind: "post", stoaAddress: spec.stoaA })
        composer.draft = "a post for A"

        composer.parentOp = "some-op"
        compare(composer.draft, "a post for A", "a parent given to a post moves nothing")
        composer.parentOp = "another-op"
        compare(composer.draft, "a post for A", "nor does a different one")
        composer.draft = "edited under another parent"
        composer.parentOp = ""
        compare(composer.draft, "edited under another parent",
                "and an edit made under one is the draft under any other")

        var sent = []
        Core.bridge = {
            callModule: function (module, method, args) {
                sent.push({ method: method, args: JSON.parse(String(args[0])) })
                return '{"opId":"newpost","wasNew":true}'
            }
        }
        composer.parentOp = "a third"
        composer.submit()
        compare(sent.length, 1)
        compare(sent[0].method, "publish_post")
        compare(sent[0].args.body, "edited under another parent")
        verify(sent[0].args.parent === undefined, "the publish names no parent")
        composer.destroy()
    }

    // Scenario: Drafts for six hundred targets are all held at once.
    // Requirement: "The view MUST NOT discard a held draft on account of how
    // many other targets hold one." The several-targets scenario names five; a
    // bound somewhere above five would pass it, so this holds hundreds.
    function test_no_held_draft_is_discarded_on_account_of_how_many_others_are_held() {
        var composer = composerComponent.createObject(null, { kind: "post" })
        var count = 600
        for (var i = 0; i < count; i++) {
            composer.stoaAddress = "stoa-" + i
            composer.draft = "draft number " + i
        }
        for (var j = 0; j < count; j++) {
            composer.stoaAddress = "stoa-" + j
            if (composer.draft !== "draft number " + j)
                fail("the draft for target " + j + " was lost; the field holds '"
                     + composer.draft + "'")
        }
        composer.destroy()
    }

    // Scenario: A post draft is back after the moderation screen was visited.
    // Requirement: "for as long as the view stays open, whichever screens are
    // rendered meanwhile." The moderation screen is a main-area screen like the
    // others; leaving the feed for it and coming back is not the end of a draft.
    function test_a_post_draft_is_back_after_the_moderation_screen_was_visited() {
        var s = spec.started()
        spec.goTo(s.view, spec.stoaA)
        spec.enter(s.view, "post", "half-written")

        spec.visibleNamed(s.view, "feed")[0].moderationRequested()
        compare(s.view.screenShown, "moderation")
        spec.visibleNamed(s.view, "moderationBackButton")[0].clicked()
        compare(s.view.screenShown, "feed")

        compare(spec.field(s.view, "post").text, "half-written")
        s.view.destroy()
    }
}
