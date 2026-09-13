import QtQuick
import QtQuick.Layouts

// The attribution unit: mark, generated name, address, badges.
// The address is never optional. A generated name is neither unique nor an
// identifier, and a MODERATOR badge beside a name with no address is precisely
// what turns a lookalike into apparent authority.
RowLayout {
    id: root

    property string identityAddress: ""
    property string generatedName: ""
    property bool   isModerator: false
    property bool   edited: false
    property bool   vouched: false          // the VIEWER's own vouch; never a count
    property bool   lookalikeWarning: false
    property int    markSize: DTheme.markInFeed

    spacing: 10

    Identicon {
        address: root.identityAddress
        size: root.markSize
        visible: root.markSize >= DTheme.markMinDraw
        Layout.alignment: Qt.AlignVCenter
    }

    Text {
        text: root.generatedName
        font: DTheme.bodySmall
        color: DTheme.ink
        textFormat: Text.PlainText       // peer-supplied: never rich text
    }

    AddressLabel {
        address: root.identityAddress
        emphasis: root.lookalikeWarning
    }

    Rectangle {                          // MODERATOR
        visible: root.isModerator
        color: DTheme.ink
        implicitWidth: modLabel.implicitWidth + 14
        implicitHeight: modLabel.implicitHeight + 4
        Text { id: modLabel; anchors.centerIn: parent; text: "MODERATOR"; font: DTheme.label; color: DTheme.paper; textFormat: Text.PlainText }
    }

    Rectangle {                          // YOU VOUCHED — visible to its owner only
        visible: root.vouched
        color: "transparent"
        border.width: DTheme.hairline
        border.color: DTheme.accent
        implicitWidth: vouchLabel.implicitWidth + 12
        implicitHeight: vouchLabel.implicitHeight + 4
        Text { id: vouchLabel; anchors.centerIn: parent; text: "YOU VOUCHED"; font: DTheme.label; color: DTheme.accent; textFormat: Text.PlainText }
    }

    Text {                               // edited — a state, never a version number
        visible: root.edited
        text: "edited"
        font: DTheme.note
        color: DTheme.accent
        textFormat: Text.PlainText
    }

    Item { Layout.fillWidth: true }
}
