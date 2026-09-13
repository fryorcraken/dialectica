import QtQuick
import QtQuick.Layouts

// Peer-supplied text, with the sanitiser's findings shown beside it.
//
// The core sanitises on the way out (dialectica-core's `sanitise` module), so
// what arrives here is already safe to render and carries two counts saying
// what had to be done. This component renders the pair.
//
// Two rules from SPEC.md, both structural rather than stylistic:
//
//   - "never bind raw peer text to a Text element with textFormat StyledText or
//     RichText" — the body below is PlainText, always, and there is no property
//     to change that.
//   - "The marker is an inline chip: mono, 1px accent border, and it must not
//     break across lines. In QML keep it as an Item inside an inline layout,
//     not as rich-text markup." — the chips are Rectangles in a RowLayout, so
//     there is no markup path for peer text to reach.
ColumnLayout {
    id: root

    // The `{text, removed, marked}` object a core reply carries. Defaulted to
    // an empty clean value so a binding that has not resolved yet renders
    // nothing rather than "undefined".
    property var value: ({ text: "", removed: 0, marked: 0 })

    property font bodyFont: DTheme.postBody
    property color bodyColor: DTheme.ink

    readonly property int removedCount: value && value.removed ? value.removed : 0
    readonly property int markedCount: value && value.marked ? value.marked : 0

    spacing: DTheme.itemGap

    Text {
        text: root.value && root.value.text !== undefined ? root.value.text : ""

        // NOT a property anyone may override. Peer-supplied text in a
        // StyledText element is the one thing SPEC.md forbids outright.
        textFormat: Text.PlainText

        font: root.bodyFont
        color: root.bodyColor
        wrapMode: Text.WordWrap
        lineHeight: DTheme.lineHeightBody
        Layout.fillWidth: true
    }

    // The chips. Only rendered when there is something to say — a chip reading
    // "0 removed" beside every ordinary post would train readers to ignore the
    // one that matters.
    RowLayout {
        visible: root.removedCount > 0 || root.markedCount > 0
        spacing: DTheme.itemGap
        Layout.fillWidth: true

        Rectangle {
            visible: root.removedCount > 0
            color: "transparent"
            border.width: DTheme.hairline
            border.color: DTheme.accent
            implicitWidth: removedLabel.implicitWidth + 12
            implicitHeight: removedLabel.implicitHeight + 6

            Text {
                id: removedLabel
                anchors.centerIn: parent
                // copy.json `sanitiser.removed`: "%1 removed"
                text: root.removedCount + " removed"
                font: DTheme.label
                color: DTheme.accent
                textFormat: Text.PlainText
            }
        }

        Rectangle {
            visible: root.markedCount > 0
            color: "transparent"
            border.width: DTheme.hairline
            border.color: DTheme.accent
            implicitWidth: markedLabel.implicitWidth + 12
            implicitHeight: markedLabel.implicitHeight + 6

            Text {
                id: markedLabel
                anchors.centerIn: parent
                // copy.json `sanitiser.markedTitle`:
                // "contains %1 marked character(s)"
                text: "contains " + root.markedCount + " marked character(s)"
                font: DTheme.label
                color: DTheme.accent
                textFormat: Text.PlainText
            }
        }

        Item { Layout.fillWidth: true }
    }
}
