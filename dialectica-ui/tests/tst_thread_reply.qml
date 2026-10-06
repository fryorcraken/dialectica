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
    //
    // The one walk in this file: every `text` an item and its descendants carry.
    // A TextField's placeholder is among them without being asked for: its
    // default style renders the prompt through a child Text (measured, by
    // putting a placeholder in the group). It reads `children` only, so text a
    // popup or an attached tooltip carries is out of its reach, which this
    // suite cannot see.
    //
    // By default it ignores `visible`, so a hidden Text is walked as well. That
    // is stricter than "rendered", for the claim walks: a statement that is
    // hidden when the test looks and shown later cannot hide in it. The
    // statement that a reply is signed is the opposite case, a positive
    // requirement on what is RENDERED, so its walks pass `renderedOnly`, which
    // skips an item that is not visible and everything under it: a caption
    // hidden with `visible: false` satisfies nothing.
    function collectTexts(item, out, renderedOnly) {
        if (renderedOnly === true && !item.visible)
            return
        if (typeof item.text === "string")
            out.push(item.text)
        for (var i = 0; i < item.children.length; i++)
            collectTexts(item.children[i], out, renderedOnly)
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

    // ---- no edit or version claim beside the reply composer ---------------
    //
    // `thread-view`'s "The text around the reply composer says nothing about
    // editing a reply or reading its earlier versions". The screen offers no
    // way to edit a reply, no method on the module surface publishes a
    // revision, and none reads a prior version. So text promising one of those
    // is false, and text denying one goes false the day it is built, so the
    // requirement forbids both.
    //
    // The spec forbids a STATEMENT, not one sentence. These tests therefore
    // match a pattern rather than looking for the removed string, so a reworded
    // promise ("Replies may be revised") or a denial ("A reply cannot be
    // edited") fails as well. The matcher flags a word, which is what fits a
    // requirement that forbids the topic in both directions: it needs no
    // denial-aware exemption, and a wrong exemption would pass the sentence it
    // was written to catch.
    //
    // **The word list is hand-maintained.** A paraphrase outside it passes, and
    // the tables in the matcher tests are the place a missed one is added.
    //
    // Two walks, for two reaches. Each group's own walk (`replyComposerOpen`,
    // `replyGateShut`) is the one with anchors, so it cannot silently collect
    // nothing. The walk beside the rows covers the whole screen except the
    // thread's rows, so text added beside the groups, in neither of them, is
    // not unguarded. The rows are left out because the fixture's root is
    // revised, so they render `edited` and "read the earlier versions": reports
    // of what an author did, and an inert affordance on a post, which the
    // requirement says it does not bear on.
    // `test_the_walk_does_not_reach_the_thread_rows_that_report_a_revision`
    // pins that exclusion.
    //
    // The requirement binds text the SCREEN authors. Text core supplies and
    // the draft the user typed are rendered in the same places and are outside
    // it, so a walk takes the strings the test itself fed the screen and does
    // not flag them. `test_text_core_supplies_and_the_draft_are_outside_the_requirement`
    // pins that scope in both directions.
    //
    // The alternatives, and what breaks without each guard below, are
    // Decision 3 of this change's design.md, which lands at
    // `openspec/changes/archive/<date>-reply-caption-no-edit/design.md`.

    // True where `text` mentions editing a reply (or amending, updating,
    // correcting, republishing, undoing, replacing, retracting, altering it, or
    // calling it immutable), with or without an `un` or `non` prefix, or
    // mentions a version of it at all, which covers both "a later version can
    // be published" and "earlier versions stay readable". Nothing the screen
    // authors in these groups has any reason to say "version".
    //
    // It is a word list and says what it misses: "Replies are permanent.",
    // "Once published a reply is final.", "Publishing is irreversible." and
    // "Replies cannot be deleted or taken back." name none of these stems and
    // pass. They are denials the requirement forbids and no list can enumerate,
    // so the list does not grow a stem for each.
    function mentionsEditOrVersion(text) {
        return /\b(un|non-?)?(edit|revis|amend|rewrit|chang|updat|modif|correct|supersed|overwrit|republish|re-publish|fix|replac|undo|redo|retract|revert|withdr[ae]w|alter|immutab)\w*|\bversions?\b|\b(newer|older)\s+(one|copy)\b/i
            .test(text)
    }

    // The matcher, pinned both ways, one test per family of statement and one
    // for what it must leave alone. A `mentionsEditOrVersion` that always
    // answered false would make the scenario tests below pass on any tree;
    // these are the tests that fail then. They are separate so that the first
    // family to fail does not hide the others.
    //
    // Every string here is written out by hand: the statements from the
    // requirement, the truthful ones as the screen's copy was when this was
    // written. None is read off the screen at run time, so the matcher is never
    // asked to agree with what the implementation produced.
    function unflagged(strings) {
        return strings.filter(function (s) { return !mentionsEditOrVersion(s) })
    }

    function test_the_matcher_flags_a_claim_that_a_reply_can_be_edited() {
        compare(unflagged([
            "A reply is a signed record. It can be edited later.",
            "Replies may be revised.",
            "You can amend this reply after publishing it.",
            "You can change it later.",
            "You can update this reply afterwards.",
            "You can modify a reply once it is published.",
            "A correction can be published.",
            "A reply can be republished with different text.",
            // Paraphrases that name none of the verbs above. The first is the
            // spec-test review's surviving caption; the others are the
            // promises a writer reaches for next.
            "A reply is a signed record. You can fix typos later and replace it.",
            "You can undo a reply after publishing it.",
            "You can redo a reply that came out wrong.",
            "A reply can be retracted.",
            "A reply can be reverted to what you wrote before.",
            "You can withdraw a reply and publish another."
        ]), [], "an edit claim the matcher let through")
    }

    // The requirement forbids a DENIAL as well as a promise: editing is planned,
    // so "cannot be edited" goes false the day it lands. A matcher that let a
    // denial through would be satisfied by exactly the sentence the owner did
    // not ask for. Hand-written, one per topic the requirement names.
    function test_the_matcher_flags_a_denial_as_well_as_a_promise() {
        compare(unflagged([
            "A reply cannot be edited.",
            "Replies cannot be changed once they are published.",
            "There is no way to edit a reply yet.",
            "A published reply cannot be undone.",
            "Editing is not available in this version.",
            "Later versions of a reply cannot be published.",
            "Earlier versions are not kept.",
            "Nothing earlier than this version can be read.",
            // A prefixed form is not a word start, and a denial may use a word
            // the edit verbs lack.
            "A reply stays unchanged once published.",
            "Replies are uneditable.",
            "Replies are noneditable.",
            "An unrevised reply stays as it was written.",
            "Replies are immutable.",
            "A reply cannot be altered."
        ]), [], "a denial the matcher let through")
    }

    function test_the_matcher_flags_a_claim_that_a_later_version_can_be_published() {
        compare(unflagged([
            "A later version of this reply can be published.",
            "Publish a new version at any time.",
            "Newer versions of a reply can be published.",
            "Another version can follow.",
            // No "version" and no edit verb: the sentence promises a newer
            // copy by naming it.
            "A newer one can take its place at any time."
        ]), [], "a later-version claim the matcher let through")
    }

    // The requirement's own clause for reading a prior version. The first
    // string is the design bundle's clause, verbatim, which stood unguarded
    // when only edit claims were forbidden.
    function test_the_matcher_flags_a_claim_that_an_earlier_version_can_be_read() {
        compare(unflagged([
            "earlier versions stay readable",
            "A reply is a signed record; earlier versions stay readable.",
            "Earlier versions of a reply can be read.",
            "You can read the previous version of a reply.",
            "Every version of a reply is kept.",
            "You can read the older one.",
            // The thread rows' own label. It is no claim inside the composer's
            // group, but the matcher must recognise it, or the scope test
            // below would prove nothing about why the walk is scoped.
            "read the earlier versions"
        ]), [], "an earlier-version claim the matcher let through")
    }

    function test_the_matcher_leaves_alone_what_the_screen_authors_in_both_gate_states() {
        // The strings the screen authors in the composer's group and in the
        // shut gate, as the fixtures drive them, including everything the
        // composer's outcome area says after a publish. Core's reason behind a
        // shut gate is not in the table: it is not the screen's copy, and the
        // scope test below is what covers it. If a sentence like these were
        // flagged, the claim tests would be reporting copy and not a claim.
        var truthful = [
            "A reply is a signed record.",
            "REPLYING AS",
            "Publish the reply",
            "42 OF 153600 BYTES",
            "You cannot reply in this Stoa yet.",
            "There is no disabled composer here. A box you could type into and not send would lose what you wrote.",
            "Your reply was saved on this machine.",
            "This reply was already published.",
            "Your reply was not published.",
            "It is in this machine's log.",
            "The identical content is already in this machine's log, under the same op id. Nothing new was written.",
            "Whether any other peer has received it is not something this software can tell you yet.",
            "Nothing was published and what you wrote is still here. You can try again.",
            "This reply is longer than the core module will accept, so it cannot be sent yet. Nothing has been removed from what you wrote.",
            "This draft contains 2 invisible character(s). They will be published exactly as you typed them, and readers' software will remove them when it renders this reply."
        ]
        compare(mentionsAmong(truthful), [],
                "a sentence making no edit or version statement, flagged")
    }

    // The texts among `texts` that the matcher flags. `suppliedText` is the
    // strings the test itself fed the screen as core's reply or as the user's
    // draft. They are outside the requirement, so they are collected and not
    // flagged. They are exact strings the test chose, so text the screen
    // authors is never exempt by accident.
    function mentionsAmong(texts, suppliedText) {
        return texts.filter(function (t) {
            return mentionsEditOrVersion(t)
                && (suppliedText === undefined || suppliedText.indexOf(t) === -1)
        })
    }

    function mentionsUnder(item, suppliedText) {
        var texts = []
        collectTexts(item, texts)
        return { texts: texts, flagged: mentionsAmong(texts, suppliedText) }
    }

    // Every text in the screen's content column outside the thread's rows. The
    // rows are `threadItems`' delegates, which a Repeater parents beside itself
    // and not under itself, in the column the screen's items are laid out in
    // (the screen's own single child), so they are named by `itemAt` rather
    // than found by structure.
    function textsBesideTheRows(screen) {
        var rows = findChild(screen, "threadItems")
        var column = rows.parent
        var skip = [rows]
        for (var r = 0; r < rows.count; r++)
            skip.push(rows.itemAt(r))
        var texts = []
        for (var c = 0; c < column.children.length; c++)
            if (skip.indexOf(column.children[c]) === -1)
                collectTexts(column.children[c], texts)
        return { rowCount: rows.count, texts: texts }
    }

    // Non-vacuity for the open group: a walk that reached the wrong item, or
    // nothing, would collect no text and report no claim. The anchors are the
    // group's two stable pieces of its own, the attribution line and the
    // composer's submit label, which together show the walk covers the group
    // and descends into the composer inside it. They are not the caption: the
    // caption's removal is the signed-statement tests' to report, and must not
    // be reported as an inability to look for a claim.
    //
    // A caption anywhere on the screen must lie inside the walked group. A
    // caption moved out of it, carrying a claim, would otherwise be text beside
    // the composer that the group's walk does not reach (the walk beside the
    // rows does, and this fails first, naming the cause).
    function verifyWalkCoversTheOpenGroup(screen, open, found) {
        verify(found.texts.indexOf("REPLYING AS") !== -1,
               "the walk reached the attribution line of the composer group")
        verify(found.texts.indexOf("Publish the reply") !== -1,
               "and went down into the composer's own submit label")
        var caption = findChild(screen, "replyCaption")
        if (caption !== null)
            verify(findChild(open, "replyCaption") === caption,
                   "a caption on the screen lies inside the group the walk covers")
    }

    function test_the_open_composers_text_makes_no_edit_or_version_claim() {
        var calls = []
        var screen = makeScreen(calls, true)

        var open = findChild(screen, "replyComposerOpen")
        verify(open !== null && open.visible, "an open gate renders the composer group")

        var found = mentionsUnder(open)
        verifyWalkCoversTheOpenGroup(screen, open, found)
        compare(found.flagged, [],
                "no text rendered with the reply composer may state whether a reply "
                + "can be edited, whether a later version of it can be published, or "
                + "whether an earlier version of it can be read")
        screen.destroy()
    }

    // The walk of a group does not reach text beside it. A Text added between
    // the two groups, or after them, is rendered with the reply composer in the
    // requirement's sense and sits in neither subtree, so the group walks pass
    // over it. This walks every text on the screen outside the thread's rows,
    // for each of the three states, so the guard is on the screen and not on two
    // containers.
    function test_no_text_beside_the_thread_rows_makes_an_edit_or_version_claim() {
        var states = [
            { name: "an open gate", canPost: true, anchor: "REPLYING AS", publish: false },
            { name: "a shut gate", canPost: false, anchor: "You cannot reply in this Stoa yet.", publish: false },
            { name: "an open gate after a publish", canPost: true, anchor: "REPLYING AS", publish: true }
        ]
        for (var s = 0; s < states.length; s++) {
            var screen = states[s].publish ? publishedScreen([]) : makeScreen([], states[s].canPost)

            var beside = textsBesideTheRows(screen)
            verify(beside.rowCount > 0,
                   states[s].name + ": the thread has rows, so there is something to leave out")
            verify(beside.texts.indexOf(states[s].anchor) !== -1,
                   states[s].name + ": the walk reached the composer's place")
            compare(beside.texts.indexOf("edited"), -1,
                    states[s].name + ": the walk leaves out the revised marker in the rows")
            compare(mentionsAmong(beside.texts), [],
                    states[s].name + ": no text beside the thread rows may state whether "
                    + "a reply can be edited, whether a later version of it can be "
                    + "published, or whether an earlier version of it can be read")
            screen.destroy()
        }
    }

    // The walks leave out the thread's own rows, because they legitimately
    // render `edited` and "read the earlier versions" on a revised post, and
    // `thread-view` says the requirement does not bear on them. This pins that
    // leaving them out is what keeps the tests above green on those two
    // strings, and not a matcher that fails to see them: the matcher flags
    // both, on the screen as a whole, and neither is in either group's walk or
    // in the walk beside the rows.
    function test_the_walk_does_not_reach_the_thread_rows_that_report_a_revision() {
        var gates = [
            { canPost: true, group: "replyComposerOpen" },
            { canPost: false, group: "replyGateShut" }
        ]
        for (var g = 0; g < gates.length; g++) {
            var calls = []
            var screen = makeScreen(calls, gates[g].canPost)
            var allTexts = []
            collectTexts(screen, allTexts)

            var wholeScreen = mentionsAmong(allTexts)
            verify(wholeScreen.indexOf("edited") !== -1,
                   "the revised marker is on screen and the matcher flags it")
            verify(wholeScreen.indexOf("read the earlier versions") !== -1,
                   "so is the earlier-versions label, and the matcher flags it")

            var group = findChild(screen, gates[g].group)
            verify(group !== null && group.visible, gates[g].group + " is rendered")
            var inGroup = mentionsUnder(group)
            compare(inGroup.flagged.indexOf("edited"), -1,
                    gates[g].group + "'s walk does not reach the revised marker")
            compare(inGroup.flagged.indexOf("read the earlier versions"), -1,
                    gates[g].group + "'s walk does not reach the earlier-versions label")
            compare(findChild(group, "earlierVersionsInert"), null,
                    "the earlier-versions row is not inside " + gates[g].group)

            var beside = textsBesideTheRows(screen)
            compare(beside.texts.indexOf("edited"), -1,
                    "the walk beside the rows does not reach the revised marker")
            compare(beside.texts.indexOf("read the earlier versions"), -1,
                    "nor the earlier-versions label")
            screen.destroy()
        }
    }

    // The walk ignores `visible` for the claim tests, which is the choice
    // `collectTexts`'s comment gives its reason for. Nothing in the screen's own
    // tree is hidden with a claim in it, so a walk narrowed to visible items
    // would pass every other test here. This one is on a hand-made tree.
    Component {
        id: hiddenClaimComponent
        Item {
            Text { text: "Replies can be edited later."; visible: false }
        }
    }

    function test_the_claim_walk_reaches_a_text_that_is_hidden() {
        var tree = hiddenClaimComponent.createObject(null)
        compare(mentionsUnder(tree).flagged, ["Replies can be edited later."],
                "a hidden Text is walked, and flagged")
        var rendered = []
        collectTexts(tree, rendered, true)
        compare(rendered, [],
                "and `renderedOnly` is what leaves it out, for the signed-statement walk")
        tree.destroy()
    }

    // The requirement binds text the screen AUTHORS. Core's reason behind a shut
    // gate, core's message on a refused publish, and the draft are rendered in
    // the same places and are not its. The fixtures elsewhere supply strings
    // that avoid the matcher, so they cannot show this. These strings are the
    // ones that would trip it: both are real core wordings (the keystore's
    // "keystore format version {v} is newer than this build understands" and the
    // identity record's layout-version refusal), and a draft that talks about
    // editing, as a person's draft may.
    function test_text_core_supplies_and_the_draft_are_outside_the_requirement() {
        var reason = "keystore format version 9 is newer than this build understands; upgrade dialectica"
        var refusal = "the identity record declares layout version 2, which this build cannot read"
        var draft = "I will edit this and publish a newer version later"

        // A shut gate rendering core's reason.
        Core.bridge = {
            callModule: function (module, method, args) {
                if (method === "get_capabilities")
                    return JSON.stringify({ canPost: false, reason: reason })
                if (method === "read_thread")
                    return JSON.stringify({ items: [rootItem()], page: 0, hasMore: false })
                return '{"error":"no fake reply for ' + method + '"}'
            }
        }
        var shutScreen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", threadId: "root1"
        })
        var shutFound = mentionsUnder(findChild(shutScreen, "replyGateShut"))
        verify(shutFound.texts.indexOf(reason) !== -1,
               "the shut gate renders core's reason verbatim")
        verify(shutFound.flagged.indexOf(reason) !== -1,
               "and the matcher does flag it, so what spares it is the scope")
        compare(mentionsUnder(findChild(shutScreen, "replyGateShut"), [reason]).flagged, [],
                "core's reason is not text the screen authors")
        // Beside the rows, the same two directions, asked of the reason alone:
        // another claim on the screen is the beside-the-rows test's to report,
        // and must not fail the scope here.
        var shutBeside = textsBesideTheRows(shutScreen).texts
        verify(mentionsAmong(shutBeside).indexOf(reason) !== -1,
               "the walk beside the rows reaches core's reason, and the matcher flags it")
        compare(mentionsAmong(shutBeside, [reason]).indexOf(reason), -1,
                "and it is not text the screen authors there either")
        shutScreen.destroy()

        // An open gate with a draft in the field and core's refusal under it.
        Core.bridge = {
            callModule: function (module, method, args) {
                if (method === "get_capabilities")
                    return '{"canPost":true,"reason":""}'
                if (method === "read_thread")
                    return JSON.stringify({ items: [rootItem()], page: 0, hasMore: false })
                if (method === "publish_reply")
                    return JSON.stringify({ error: refusal })
                return '{"error":"no fake reply for ' + method + '"}'
            }
        }
        var openScreen = threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", threadId: "root1"
        })
        var composer = findChild(openScreen, "replyComposer")
        composer.draft = draft
        composer.submit()
        compare(composer.outcome, "refused", "the publish was refused, so the draft stays")

        var open = findChild(openScreen, "replyComposerOpen")
        var openFound = mentionsUnder(open)
        verify(openFound.texts.indexOf(draft) !== -1, "the walk collects the draft")
        verify(openFound.texts.indexOf(refusal) !== -1, "and core's refusal, verbatim")
        verify(openFound.flagged.indexOf(draft) !== -1
               && openFound.flagged.indexOf(refusal) !== -1,
               "and the matcher does flag both, so what spares them is the scope")
        compare(mentionsUnder(open, [draft, refusal]).flagged, [],
                "neither is text the screen authors")
        var openBeside = textsBesideTheRows(openScreen).texts
        var flaggedBeside = mentionsAmong(openBeside)
        verify(flaggedBeside.indexOf(draft) !== -1 && flaggedBeside.indexOf(refusal) !== -1,
               "the walk beside the rows reaches both, and the matcher flags them")
        var exemptedBeside = mentionsAmong(openBeside, [draft, refusal])
        compare(exemptedBeside.indexOf(draft), -1, "the draft is not text the screen authors there either")
        compare(exemptedBeside.indexOf(refusal), -1, "nor is core's refusal")
        openScreen.destroy()
    }

    // A screen with a reply published through its composer, and the re-read it
    // triggers. The composer stored the op and the thread holds the reply.
    // Both are asserted here, so a caller never walks the pre-publish tree and
    // passes: a fixture whose `publish_reply` and `read_thread` stop agreeing
    // fails loudly on these two lines.
    function publishedScreen(calls) {
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

        compare(composer.outcome, "stored",
                "the publish went through the composer and was newly stored")
        compare(screen.items.length, 2,
                "the thread was read again and holds the published reply")
        return screen
    }

    // The caption stays on screen across a publish, so the check is repeated
    // after one, with the re-read returning the published reply.
    function test_no_edit_or_version_claim_appears_after_a_reply_is_published() {
        var screen = publishedScreen([])
        var open = findChild(screen, "replyComposerOpen")
        verify(open !== null && open.visible, "the composer group is still rendered")

        var found = mentionsUnder(open)
        verifyWalkCoversTheOpenGroup(screen, open, found)
        compare(found.flagged, [],
                "after a publish, no text rendered with the reply composer may "
                + "state whether the reply can be edited, whether a later version of "
                + "it can be published, or whether an earlier version of it can be read")
        screen.destroy()
    }

    function test_the_shut_gates_text_makes_no_edit_or_version_claim() {
        var calls = []
        var screen = makeScreen(calls, false)

        var shut = findChild(screen, "replyGateShut")
        verify(shut !== null && shut.visible, "a shut gate renders in the composer's place")

        // Non-vacuity, as above: the gate's own heading is in the walk.
        var found = mentionsUnder(shut)
        verify(found.texts.indexOf("You cannot reply in this Stoa yet.") !== -1,
               "the walk reached the shut gate's text")
        compare(found.flagged, [],
                "no text rendered in place of the reply composer may state whether a "
                + "reply can be edited, whether a later version of it can be published, "
                + "or whether an earlier version of it can be read")
        screen.destroy()
    }

    // ---- the open composer states that a reply is signed ------------------
    //
    // `thread-view`'s "The open reply composer states that a reply is signed":
    // text rendered with the composer states it, and still does after a
    // publish. The obligation is on the statement, not on a string, so these
    // look for the statement in whatever the group renders (`renderedOnly`: a
    // caption that is not shown states nothing) and not for the caption by
    // name. A caption cut to unrelated copy, deleted, or hidden fails them.
    //
    // The matcher is hand-written like the other one, and pinned the same way,
    // both directions, below. It is a pattern and not a reading of the
    // sentence: a text counts when it says `signed` and `reply` (or `replies`)
    // and has no negator (`not`, `never`, `no`, `cannot`, `n't`) within two
    // words before `signed` in the same clause. A paraphrase that says the
    // opposite without a negator ("A reply is unverifiable"), or one with a
    // negator further off ("Not every reply you publish here is signed"),
    // passes as a statement; so does a sentence about something else that
    // happens to name a reply and say `signed`. The table is where such a miss
    // is added.
    function statesReplyIsSigned(text) {
        return /\bsigned\b/i.test(text)
            && /\brepl(y|ies)\b/i.test(text)
            && !/(\bnot\b|\bnever\b|\bno\b|\bcannot\b|n't)(\s+\w+){0,2}\s+signed\b/i.test(text)
    }

    function test_the_signed_matcher_accepts_what_states_it_and_refuses_what_does_not() {
        var states = [
            "A reply is a signed record.",
            "Every reply you publish is signed with your key.",
            "Replies are signed.",
            "Your reply will be signed by the key above.",
            // A negation AFTER `signed` is not a denial of it.
            "A reply is a signed record, not a private message."
        ]
        var notStates = [
            "A reply is not signed.",
            "Replies are never signed.",
            "A reply isn't signed.",
            "An unsigned reply.",
            "A signature is not required.",
            // A negation the clause once could not see: `cannot` has no word
            // boundary before `not`, and a word between negator and `signed`
            // defeated a closed list. The first is the spec-test review's
            // surviving caption.
            "A reply cannot be signed.",
            "Your reply is not cryptographically signed.",
            "Replies are not always signed.",
            "Replies can't be signed.",
            // `signed`, said of something that is not a reply.
            "Signed in as alice.",
            "Your draft is signed off by the app.",
            "REPLYING AS",
            "Publish the reply",
            "Your reply was saved on this machine.",
            ""
        ]
        compare(notStates.filter(statesReplyIsSigned), [],
                "a text taken for a statement that a reply is signed")
        compare(states.filter(function (s) { return !statesReplyIsSigned(s) }), [],
                "a statement that a reply is signed, not recognised")
    }

    // The rendered texts of a group that state a reply is signed.
    function signedStatementsIn(group) {
        var texts = []
        collectTexts(group, texts, true)
        return { texts: texts, statements: texts.filter(statesReplyIsSigned) }
    }

    function test_the_open_composers_text_states_that_a_reply_is_signed() {
        var calls = []
        var screen = makeScreen(calls, true)

        var open = findChild(screen, "replyComposerOpen")
        verify(open !== null && open.visible, "an open gate renders the composer group")

        var found = signedStatementsIn(open)
        // The anchor is the attribution line, which is rendered whatever the
        // draft, unlike the submit label that appears once there is one.
        verify(found.texts.indexOf("REPLYING AS") !== -1,
               "the walk reached the composer group's rendered text")
        verify(found.statements.length > 0,
               "text rendered with the reply composer states that a reply is signed")
        screen.destroy()
    }

    function test_the_statement_that_a_reply_is_signed_survives_a_publish() {
        var screen = publishedScreen([])

        var open = findChild(screen, "replyComposerOpen")
        verify(open !== null && open.visible, "the composer group is still rendered")

        var found = signedStatementsIn(open)
        verify(found.texts.indexOf("REPLYING AS") !== -1,
               "the walk reached the composer group's rendered text")
        verify(found.texts.indexOf("Your reply was saved on this machine.") !== -1,
               "and the group is the one the publish reported into")
        verify(found.statements.length > 0,
               "text rendered with the reply composer still states that a reply is signed")
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
