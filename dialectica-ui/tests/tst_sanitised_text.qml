import QtQuick
import QtTest
import "../src/qml"

// `SanitisedText` renders what core handed it, and does not re-interpret it.
//
// Core sanitises on the way out and sends `{text, removed, marked}`. The view's
// job is narrow and the two halves of it are both easy to get wrong:
//
//   - render the text as PLAIN TEXT, always, whatever it contains;
//   - show the chips only when there is something to report.
//
// The security-relevant half is the first. SPEC.md forbids binding peer text to
// a Text element with StyledText or RichText, and QML's DEFAULT is AutoText,
// which sniffs its input — so "it renders fine today" is not evidence.
TestCase {
    id: spec
    name: "SanitisedText"

    Component {
        id: sanitisedComponent
        SanitisedText {}
    }

    function make(value) {
        return sanitisedComponent.createObject(null, { value: value })
    }

    // The body Text is the first child of the ColumnLayout root.
    function bodyOf(item) {
        for (var i = 0; i < item.children.length; i++) {
            if (item.children[i].textFormat !== undefined)
                return item.children[i]
        }
        return null
    }

    // ---- the text is bound verbatim, as plain text ----------------------

    function test_the_body_renders_as_plain_text_whatever_it_contains() {
        // THE invariant. Markup in a post body must reach the screen as the
        // characters the author published, never as formatting. Under AutoText
        // this string would render as bold with the tags eaten.
        var item = make({ text: "<b>not bold</b> & <i>not italic</i>",
                          removed: 0, marked: 0 })
        var body = bodyOf(item)

        verify(body !== null, "the body Text must exist")
        compare(body.textFormat, Text.PlainText,
                "peer text must be PlainText — AutoText sniffs for markup")
        compare(body.text, "<b>not bold</b> & <i>not italic</i>",
                "the text must be bound verbatim, tags and all")
        item.destroy()
    }

    function test_the_body_is_not_plain_text_by_coincidence_of_content() {
        // The same assertion against content with NO markup in it, so the
        // property is pinned as a property of the element rather than a
        // consequence of what this fixture happened to contain.
        var item = make({ text: "an ordinary sentence", removed: 0, marked: 0 })
        compare(bodyOf(item).textFormat, Text.PlainText)
        item.destroy()
    }

    function test_a_homoglyph_reaches_the_screen_unchanged() {
        // Core marks and never corrects, because correcting changes what the
        // record says. The view must not undo that by normalising either.
        var withCyrillic = "pаypal"
        var item = make({ text: withCyrillic, removed: 0, marked: 1 })
        compare(bodyOf(item).text, withCyrillic,
                "the view must not correct what core deliberately preserved")
        item.destroy()
    }

    // ---- the chips report what core found, and nothing else ------------

    function test_a_clean_string_shows_no_chip() {
        // A chip on every ordinary post trains readers to ignore the one that
        // matters.
        var item = make({ text: "ordinary", removed: 0, marked: 0 })
        compare(item.removedCount, 0)
        compare(item.markedCount, 0)
        item.destroy()
    }

    function test_the_counts_come_from_the_reply_rather_than_being_recomputed() {
        // The view must not have its own opinion about what is suspicious:
        // core decided, and a second implementation here could disagree with
        // it. The text below contains NO invisible characters, so a view
        // counting for itself would report zero.
        var item = make({ text: "perfectly ordinary text", removed: 3, marked: 7 })
        compare(item.removedCount, 3, "the view must report core's count")
        compare(item.markedCount, 7)
        item.destroy()
    }

    function test_a_missing_value_renders_empty_rather_than_undefined() {
        // A binding that has not resolved yet must not put the string
        // "undefined" on screen.
        var item = sanitisedComponent.createObject(null, {})
        compare(bodyOf(item).text, "")
        compare(item.removedCount, 0)
        compare(item.markedCount, 0)
        item.destroy()
    }

    function test_a_value_missing_its_counts_is_treated_as_clean() {
        var item = make({ text: "just text" })
        compare(bodyOf(item).text, "just text")
        compare(item.removedCount, 0)
        compare(item.markedCount, 0)
        item.destroy()
    }
}
