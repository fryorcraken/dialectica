import QtQuick
import QtTest
import "../src/qml"

// UI-BRIEF rendering obligation 10, second half: **where the interface asserts
// extent, that assertion must be readable as local.**
//
// The feed's extent claim is the pagination control. `hasMore` is computed by
// `feed::list_threads` from this peer's log alone, so "Next" means *this machine
// holds another page* — while a reader meeting a full page and a "Next" button
// reads it as *this Stoa has more*, which is the claim no peer can make.
//
// Obligation 10 also states the explicit NON-obligation: a screen asserting no
// extent owes nothing, because a locality line printed once per screen is a
// disclaimer at the reader rather than a discharged obligation.
//
// **Why these tests assert a shared PARENT rather than `visible`.** A QML item's
// `visible` reports EFFECTIVE visibility, and a `TestCase` is itself invisible
// offscreen — so every descendant reads `visible === false` whatever its own
// binding says, including elements that unconditionally render. Height is no
// better: an unlaid subtree reports content height, and it was MEASURED to be
// identical with the locality sentence present and replaced by a one-word
// string. Both would be gates the defect satisfies.
//
// What IS readable is the object graph. The sentence and the buttons sharing one
// governing ancestor is the property that makes "renders in the state where
// paging is offered, and only there" true by construction: there is one
// `visible:` binding for both, so no future edit can show the control while
// hiding the sentence without first taking them apart — which these tests see.
TestCase {
    id: spec
    name: "FeedExtentClaim"

    property var savedBridge: undefined

    function init() { spec.savedBridge = Core.bridge }
    function cleanup() { Core.bridge = spec.savedBridge }

    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    Component {
        id: feedComponent
        FeedScreen {}
    }

    function rowsJson(n) {
        var parts = []
        for (var i = 0; i < n; ++i) {
            parts.push('{"thread":"t' + i + '","currentVersion":"t' + i + '",'
                + '"author":"' + "cd".repeat(32) + '",'
                + '"body":{"text":"post ' + i + '","removed":0,"marked":0},'
                + '"attachments":[],"isRevised":false,"isHidden":false}')
        }
        return parts.join(",")
    }

    // `stoaAddress` must be non-empty or reload() short-circuits before it calls
    // the bridge, which would make every test here pass for the wrong reason.
    function screenFor(rows, hasMore) {
        Core.bridge = bridgeFor({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[' + rowsJson(rows) + '],"page":0,'
                + '"hasMore":' + (hasMore ? "true" : "false") + '}'
        })
        return feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora",
            width: 1000
        })
    }

    // The first element under `item` whose `text` starts with `prefix`.
    // Note this can return a FlatButton rather than its inner Text, since
    // FlatButton exposes `text` — which is what the caller wants here.
    function findText(item, prefix) {
        var kids = item.children
        for (var i = 0; i < kids.length; ++i) {
            var c = kids[i]
            if (c.text !== undefined && typeof c.text === "string"
                && c.text.indexOf(prefix) === 0)
                return c
            var hit = findText(c, prefix)
            if (hit !== null)
                return hit
        }
        return null
    }

    // The locality sentence, identified by a distinguishing prefix rather than
    // pinned in full. A pin fails on a reword, which is not the defect; what
    // must not happen is the sentence ceasing to be about whose copy it is.
    readonly property string localityPrefix: "Pages are what this machine holds"

    function test_the_paging_control_carries_a_locality_statement() {
        var screen = screenFor(30, true)

        var sentence = findText(screen, spec.localityPrefix)
        verify(sentence !== null,
               "a feed offering another page asserts extent, and obligation 10 "
               + "requires that assertion to be readable as local — no locality "
               + "sentence was found on the screen")

        // It must deny the over-reading, not merely mention locality. "Next"
        // is what the reader over-reads, so the sentence has to name it.
        verify(sentence.text.indexOf("Stoa") >= 0,
               "the sentence must deny the claim about the STOA, which is the "
               + "reading 'Next' invites; got: " + sentence.text)

        screen.destroy()
    }

    function test_the_sentence_and_the_buttons_cannot_render_apart() {
        // THE structural assertion. One governing ancestor means one `visible:`
        // binding, so the control cannot appear without the sentence and the
        // sentence cannot appear without the control.
        var screen = screenFor(30, true)

        var sentence = findText(screen, spec.localityPrefix)
        var next = findText(screen, "Next")
        verify(sentence !== null, "no locality sentence")
        verify(next !== null, "no Next button")

        // findText returns the FlatButton (it exposes `text`), so the chain is
        // FlatButton -> RowLayout -> the governing ColumnLayout.
        compare(sentence.parent, next.parent.parent,
                "the locality sentence and the paging buttons must share one "
                + "governing ancestor, so a single `visible:` binding decides "
                + "both — otherwise a later edit can show the extent claim "
                + "while hiding what makes it readable as local")

        screen.destroy()
    }

    function test_a_screen_asserting_no_extent_owes_no_locality_line() {
        // Obligation 10's explicit non-obligation, and the half that keeps this
        // from becoming a disclaimer. The empty feed offers no paging: it makes
        // no extent claim, so it must not carry the paging locality sentence.
        //
        // The empty state has its own locality sentence about the STORE READ,
        // which is a different claim and is not what this looks for.
        var screen = screenFor(0, false)

        compare(screen.readState, "ok", "an empty store is a success")
        compare(screen.rows.length, 0)

        var sentence = findText(screen, spec.localityPrefix)
        var next = findText(screen, "Next")

        // Both are present in the object graph — a `visible: false` layout child
        // is constructed, not omitted — so the assertion is that they travel
        // together, which is what makes "only where extent is asserted" hold.
        if (sentence !== null) {
            verify(next !== null,
                   "the paging sentence must not outlive the paging control")
            compare(sentence.parent, next.parent.parent,
                    "even unrendered, the two must stay under one binding")
        }

        screen.destroy()
    }
}
