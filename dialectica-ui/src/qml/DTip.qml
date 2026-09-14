import QtQuick
import QtQuick.Controls

// A tooltip that renders its text as PLAIN TEXT, and the only tooltip this tree
// is allowed to use.
//
// WHY THIS COMPONENT EXISTS AT ALL, because `ToolTip.text: someString` on a
// MouseArea is one line and this is fifteen.
//
// A `ToolTip`'s default content item is a `Text` whose `textFormat` is
// `Text.StyledText` (2), NOT PlainText — Controls sets it explicitly, so this is
// not QML's AutoText default and reasoning about AutoText does not reach it. A
// tooltip is therefore a markup sink that no `textFormat:` appears anywhere near,
// and the strings that reach one here are the sentences core supplies to explain
// a degraded lamp: exactly where an attacker-influenced fragment (a peer address,
// a relay-supplied reason) ends up.
//
// MEASURED on Qt 6.10.3, because "a tooltip is probably plain" is the assumption
// this replaces. Against `ToolTip.text: "<b>OWNED</b> x"`:
//
//     attached form   contentItem.textFormat = 2   contentWidth = 56.66
//     this component  contentItem.textFormat = 0   contentWidth = 102.02
//
// and a `Text.PlainText` element painting the same string measures 102.02. The
// tags are CONSUMED in the first and DRAWN in the second. Note what the two forms
// agree on: `contentItem.text` returns `"<b>OWNED</b> x"` verbatim either way, so
// an assertion that reads the string back cannot tell them apart. Only
// `textFormat` can, which is what the tooltip tests assert.
//
// WHY NOT FIX THE ATTACHED FORM IN PLACE. `ToolTip.text` on an item routes
// through a SHARED tooltip instance owned by the attached type, so there is no
// per-site content item to give a format to — the format can only be pinned by
// declaring the tooltip, which is what this does. Every tooltip in
// `dialectica-ui/src/qml` is therefore an explicit `DTip`, and CI's
// `no bare ToolTip.text` step keeps it that way: the attached form is banned
// outright rather than audited case by case, because a tooltip binding a literal
// today is one edit away from binding a peer-supplied string, and that edit has
// no reason to think it touched rendering.
//
// WHY `DTip` AND NOT `DToolTip`, which is the name this wants. `check_qml_names`
// derives its stale-reference ban from `qmldir`: for every type declared `DX`, a
// bare `X` anywhere in a `.qml` body is an error, because that is what a
// half-finished rename of OUR type looks like. Naming this `DToolTip` therefore
// bans the bare word `ToolTip` tree-wide — including in this file, which must
// name the Qt Controls type it derives from, and in the tests, which must say
// what they are asserting about. The gate is right about our own renames and has
// no way to know that this particular `X` is upstream's type rather than a stale
// spelling of ours. Weakening it to find out would cost more than the name does,
// so the name moved. Measured: the gate reported four errors under `DToolTip`
// and reports none under `DTip`.
ToolTip {
    id: control

    // The delay the attached form defaults to, kept so replacing one with the
    // other does not change how the interface feels.
    delay: 500

    contentItem: Text {
        text: control.text
        font: DTheme.bodySmall
        color: DTheme.paper

        // THE WHOLE POINT OF THIS FILE. Do not remove, and do not "simplify"
        // this element away — the default content item is StyledText.
        textFormat: Text.PlainText
    }

    background: Rectangle {
        color: DTheme.inkSoft
        border.width: DTheme.hairline
        border.color: DTheme.ink
    }
}
