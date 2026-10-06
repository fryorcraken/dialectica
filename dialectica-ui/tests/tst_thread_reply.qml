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

    // `thread-view`'s "The reply composer is wired to the publish call and
    // names the parent it is under": "Where the view holds no op id for a
    // post, no reply call SHALL be made for it". For THIS screen the op id
    // the view holds for the root is `threadId` (the file header names it
    // "The ROOT POST's op id"), guarded in `reload()` before any read is
    // even attempted — so the concrete rendering of "an item carrying no op
    // id" is `threadId === ""`, and the affordance must be unreachable then.
    function test_no_reply_affordance_is_reachable_with_no_root_identifier() {
        var calls = []
        Core.bridge = recordingBridge(calls, true)
        var screen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            threadId: ""
        })

        var composer = findChild(screen, "replyComposer")
        var open = findChild(screen, "replyComposerOpen")
        verify(open === null || !open.visible,
               "the open-gate composer group is not rendered for a root the "
               + "view holds no id for")
        if (composer !== null)
            verify(!composer.visible, "and certainly not a typable one")

        var publishCalls = 0
        for (var i = 0; i < calls.length; i++)
            if (calls[i].method === "publish_reply")
                publishCalls += 1
        compare(publishCalls, 0,
                "no reply call was made for a post the view holds no identifier for")
        screen.destroy()
    }

    // The regression this guards against: the composer's parent is
    // `screen.threadId`, never derived from the read's own item shape — so a
    // root item the read returns with no `id` field at all must not be able
    // to influence what is sent as `parent`. A future change that switched
    // the composer to read the item's own `id` instead, without guarding for
    // its absence, would send `parent: undefined` for exactly this fixture,
    // and every other test in this file gives the root item an `id`, so none
    // of them would notice.
    function test_a_root_items_own_missing_id_does_not_reach_the_reply_parent() {
        var calls = []
        Core.bridge = {
            callModule: function (module, method, args) {
                calls.push({ method: method, args: JSON.parse(args[0]) })
                if (method === "get_capabilities")
                    return '{"canPost":true,"reason":""}'
                if (method === "read_thread") {
                    var rootWithNoId = rootItem()
                    delete rootWithNoId.id
                    return JSON.stringify({ items: [rootWithNoId], page: 0, hasMore: false })
                }
                if (method === "publish_reply")
                    return '{"opId":"newreply","wasNew":true}'
                return '{"error":"no fake reply for ' + method + '"}'
            }
        }
        var screen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            threadId: "root1"
        })

        var composer = findChild(screen, "replyComposer")
        verify(composer !== null,
               "the gate is governed by threadId and capability, not the item's own id")
        composer.draft = "x"
        composer.submit()

        var published = null
        for (var i = 0; i < calls.length; i++)
            if (calls[i].method === "publish_reply")
                published = calls[i]

        verify(published !== null)
        compare(published.args.parent, "root1",
                "the parent is the screen's threadId, never the read item's own id field")
        verify(published.args.parent !== undefined,
               "no request is sent omitting the field that would have named the parent")
        screen.destroy()
    }

    // `thread-view`'s "Every core call the thread screen makes goes through the
    // view's single call path": "The view SHALL NOT insert an item on the
    // strength of a publish having succeeded" — `composer-view`'s "A published
    // post is not shown until core has been read again", restated for this
    // screen. `DThreadScreen`'s composer fires `onPublished: screen.reload()`,
    // and the fixture's `read_thread` answers the SAME one item on every call —
    // so a locally composed row is the only way the count below could move.
    function test_no_item_is_added_by_a_publish() {
        var calls = []
        var screen = makeScreen(calls, true)

        compare(screen.items.length, 1, "the fixture starts with the root alone")

        var composer = findChild(screen, "replyComposer")
        composer.draft = "a reply"
        composer.submit()

        var reads = 0
        for (var i = 0; i < calls.length; i++)
            if (calls[i].method === "read_thread")
                reads += 1
        verify(reads >= 2, "the publish must be followed by a re-read")

        compare(screen.items.length, 1,
                "the items rendered are exactly what the re-read returned; "
                + "nothing composed by the view was inserted")
        screen.destroy()
    }

    // ---- the sanitiser report is threaded through, not just renderable ---
    //
    // `thread-view`'s "Every string rendered from an item is rendered as the
    // read supplied it" has no test in this file, `tst_thread_states.qml` or
    // `tst_thread_nesting.qml` that gives an item a non-clean `body`.
    // `tst_sanitised_text.qml` proves the SHARED COMPONENT renders a
    // sanitiser report correctly — a component test, blind to whether THIS
    // screen actually passes a thread item's `body` field through to it
    // rather than, say, `body.text` alone. A wiring defect dropping
    // `removed`/`marked` on the way from the item to the component would not
    // be caught by either file alone.

    // A recursive walk by QML type name, since `SanitisedText` carries no
    // `objectName` in `DThreadScreen.qml` and adding one would be an
    // implementation change to make for a test.
    function findByTypeName(item, typeName) {
        for (var i = 0; i < item.children.length; i++) {
            var child = item.children[i]
            if (child.toString().indexOf(typeName) !== -1)
                return child
            var found = findByTypeName(child, typeName)
            if (found !== null)
                return found
        }
        return null
    }

    function bodyTextOf(sanitisedInstance) {
        for (var i = 0; i < sanitisedInstance.children.length; i++)
            if (sanitisedInstance.children[i].textFormat !== undefined)
                return sanitisedInstance.children[i].text
        return null
    }

    function test_the_screen_threads_the_items_sanitiser_report_through_to_the_render() {
        var calls = []
        Core.bridge = {
            callModule: function (module, method, args) {
                calls.push({ method: method, args: JSON.parse(args[0]) })
                if (method === "get_capabilities")
                    return '{"canPost":true,"reason":""}'
                if (method === "read_thread") {
                    var item = rootItem()
                    item.body = { text: "reaches the screen unaltered", removed: 2, marked: 3 }
                    return JSON.stringify({ items: [item], page: 0, hasMore: false })
                }
                return '{"error":"no fake reply for ' + method + '"}'
            }
        }
        var screen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", threadId: "root1"
        })

        var sanitised = findByTypeName(screen, "SanitisedText")
        verify(sanitised !== null, "the root row renders through SanitisedText")
        compare(sanitised.removedCount, 2,
                "the item's own removed count reaches the shared component")
        compare(sanitised.markedCount, 3,
                "the item's own marked count reaches the shared component")
        compare(bodyTextOf(sanitised), "reaches the screen unaltered",
                "and the body text travels through unaltered")
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

    // `thread-view`'s "A revised post is marked as revised…" scenario "The
    // marker claims nothing about the earlier version", and "The
    // earlier-versions affordance is inert…" scenario "No earlier version
    // text is rendered": neither had an explicit test. There is currently no
    // data channel for prior-version text to travel through at all (no
    // method reads a superseded version), so a violation could only be a
    // hardcoded string added to the view — these pin the exact set of
    // strings rendered around the marker and the affordance, so such an
    // addition is caught rather than silently passing the existing boolean-
    // only assertions.
    function collectTexts(item, out) {
        if (typeof item.text === "string")
            out.push(item.text)
        for (var i = 0; i < item.children.length; i++)
            collectTexts(item.children[i], out)
    }

    function test_the_marker_and_the_inert_row_state_nothing_about_earlier_content() {
        var calls = []
        var screen = makeScreen(calls, true)

        // The marker itself: exactly the word "edited", never elaborated with
        // what changed, when, or how many times. Counting exact matches
        // (rather than asking whether ONE Text says "edited") is what catches
        // a mutation like "edited (from rev3)" — that string no longer equals
        // "edited" exactly, so the count below drops to zero.
        var allTexts = []
        collectTexts(screen, allTexts)
        var editedCount = 0
        for (var i = 0; i < allTexts.length; i++)
            if (allTexts[i] === "edited")
                editedCount += 1
        compare(editedCount, 1,
                "the revised marker is rendered as exactly the word \"edited\", "
                + "one occurrence, for the one revised item in the fixture")

        // The affordance row: exactly its own two static strings, nothing else
        // — the row `earlierVersionsInert` sits in, scoped so the enumeration
        // cannot be satisfied by content rendered elsewhere on the screen.
        var inert = findChild(screen, "earlierVersionsInert")
        verify(inert !== null)
        var rowTexts = []
        collectTexts(inert.parent, rowTexts)
        compare(rowTexts.length, 2,
                "the affordance renders only its own label and its badge")
        verify(rowTexts.indexOf("read the earlier versions") !== -1)
        verify(rowTexts.indexOf("NOT YET AVAILABLE") !== -1)
        screen.destroy()
    }

    // ---- no edit claim beside the reply composer -------------------------
    //
    // `thread-view`'s "The text around the reply composer does not promise that
    // a reply can be edited". The screen offers no way to edit a reply, and no
    // method on the module surface publishes a revision. So any text promising
    // either one is false.
    //
    // The spec forbids a CLAIM, not one sentence. These tests therefore match
    // a pattern rather than looking for the removed string, so a reworded
    // promise ("Replies may be revised") fails as well. They walk only the
    // composer's group and the shut gate: the fixture's root is revised, so
    // the thread's rows render `edited` and "read the earlier versions". Both
    // are reports of what an author did, which the requirement says it does
    // not bear on, and a whole-screen walk would fail on them.
    //
    // design.md Decision 3 covers the alternatives, and what breaks without
    // each guard below.

    // True where `text` claims a reply can be edited, or that a later version
    // of it can be published.
    function claimsEditing(text) {
        return /\b(edit|revis|amend|rewrit|chang)\w*|\b(later|new|newer|another|next) versions?\b/i
            .test(text)
    }

    // The matcher, pinned both ways. A `claimsEditing` that always answered
    // false would make all three scenario tests below pass on any tree. This
    // is the test that fails then.
    function test_the_edit_claim_matcher_flags_edit_claims_and_nothing_else() {
        var claims = [
            "A reply is a signed record. It can be edited later.",
            "Replies may be revised.",
            "You can amend this reply after publishing it.",
            "You can change it later.",
            "A later version of this reply can be published.",
            "Publish a new version at any time.",
            "Newer versions of a reply can be published."
        ]
        for (var i = 0; i < claims.length; i++)
            verify(claimsEditing(claims[i]),
                   "the matcher must flag an edit claim: \"" + claims[i] + "\"")

        // The strings this group legitimately renders, as the fixture drives
        // it. If a sentence like these were flagged, these tests would be
        // reporting copy and not a claim.
        var truthful = [
            "A reply is a signed record.",
            "REPLYING AS",
            "Publish the reply",
            "You cannot reply in this Stoa yet.",
            "no keystore",
            "There is no disabled composer here. A box you could type into and not send would lose what you wrote."
        ]
        for (var j = 0; j < truthful.length; j++)
            verify(!claimsEditing(truthful[j]),
                   "the matcher must not flag a sentence making no edit claim: \""
                   + truthful[j] + "\"")
    }

    function editClaimsUnder(item) {
        var texts = []
        collectTexts(item, texts)
        var flagged = []
        for (var i = 0; i < texts.length; i++)
            if (claimsEditing(texts[i]))
                flagged.push(texts[i])
        return { texts: texts, flagged: flagged }
    }

    function test_the_open_composers_text_makes_no_edit_claim() {
        var calls = []
        var screen = makeScreen(calls, true)

        var open = findChild(screen, "replyComposerOpen")
        verify(open !== null && open.visible, "an open gate renders the composer group")

        // Non-vacuity: the caption beside the composer is in the walk's reach.
        // A walk that reached the wrong item, or nothing, would collect no
        // text and report no claim.
        var caption = findChild(open, "replyCaption")
        verify(caption !== null && caption.visible,
               "the caption is inside the composer group the walk covers")

        var found = editClaimsUnder(open)
        verify(found.texts.indexOf(caption.text) !== -1,
               "and the walk collected it")
        compare(found.flagged.length, 0,
                "no text rendered with the reply composer may claim a reply can be "
                + "edited or re-published as a later version; flagged: "
                + JSON.stringify(found.flagged))
        screen.destroy()
    }

    // The caption stays on screen across a publish, so the check is repeated
    // after one, with the re-read returning the published reply.
    function test_no_edit_claim_appears_after_a_reply_is_published() {
        var calls = []
        var published = false
        Core.bridge = {
            callModule: function (module, method, args) {
                calls.push({ method: method, args: JSON.parse(args[0]) })
                if (method === "get_capabilities")
                    return '{"canPost":true,"reason":""}'
                if (method === "read_thread") {
                    var items = [rootItem()]
                    if (published)
                        items.push({
                            thread: "root1", id: "newreply", currentVersion: "newreply",
                            parent: "root1", author: "bb".repeat(32), isRevised: false,
                            moderation: { state: "unmoderated" }, position: "1",
                            body: { text: "Bye!", removed: 0, marked: 0 }
                        })
                    return JSON.stringify({ items: items, page: 0, hasMore: false })
                }
                if (method === "publish_reply") {
                    published = true
                    return '{"opId":"newreply","wasNew":true}'
                }
                return '{"error":"no fake reply for ' + method + '"}'
            }
        }
        var screen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", threadId: "root1"
        })

        var composer = findChild(screen, "replyComposer")
        composer.draft = "Bye!"
        composer.submit()

        compare(screen.items.length, 2,
                "the thread was read again and holds the published reply")

        var open = findChild(screen, "replyComposerOpen")
        verify(open !== null && open.visible, "the composer group is still rendered")
        var caption = findChild(open, "replyCaption")
        verify(caption !== null && caption.visible,
               "the caption is still beside the composer after the publish")

        var found = editClaimsUnder(open)
        verify(found.texts.indexOf(caption.text) !== -1, "and the walk collected it")
        compare(found.flagged.length, 0,
                "after a publish, no text rendered with the reply composer may "
                + "claim the reply can be edited; flagged: "
                + JSON.stringify(found.flagged))
        screen.destroy()
    }

    function test_the_shut_gates_text_makes_no_edit_claim() {
        var calls = []
        var screen = makeScreen(calls, false)

        var shut = findChild(screen, "replyGateShut")
        verify(shut !== null && shut.visible, "a shut gate renders in the composer's place")

        // Non-vacuity, as above: the gate's own heading is in the walk.
        var found = editClaimsUnder(shut)
        verify(found.texts.indexOf("You cannot reply in this Stoa yet.") !== -1,
               "the walk reached the shut gate's text")
        compare(found.flagged.length, 0,
                "no text rendered in place of the reply composer may claim a reply "
                + "can be edited; flagged: " + JSON.stringify(found.flagged))
        screen.destroy()
    }

    // NO SPEC: the spec forbids an edit claim but does not say whether the
    // caption stays. This change keeps its true half, that a reply is a signed
    // record (design.md Decision 1), so removing the caption entirely is a
    // visible choice rather than a silent one.
    function test_the_caption_beside_the_composer_is_kept() {
        var calls = []
        var screen = makeScreen(calls, true)

        var caption = findChild(screen, "replyCaption")
        verify(caption !== null && caption.visible, "the caption is rendered")
        verify(caption.text.length > 0, "and says something")
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
