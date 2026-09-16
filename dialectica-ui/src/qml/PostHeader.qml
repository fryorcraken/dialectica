import QtQuick
import QtQuick.Layouts

// The attribution unit: mark, generated name, public key, badges.
// The key is never optional. A generated name is neither unique nor an
// identifier, and a MODERATOR badge beside a name with no key is precisely
// what turns a lookalike into apparent authority.
RowLayout {
    id: root

    // The author's PUBLIC KEY, hex — what `author` carries on a feed row and a
    // thread item. It was `identityAddress` and held an author address; issue #80
    // deleted that value, and the property was renamed rather than left holding
    // something its name denies.
    //
    // `Identicon` and `AddressLabel` below keep an `address` property, and that is
    // deliberate rather than an oversight: both are GENERIC components that also
    // render Stoa addresses, which survive. Their own comments say so. What flows
    // into them here is a key.
    property string identityKey: ""
    property string generatedName: ""
    property bool   isModerator: false
    property bool   edited: false
    property bool   vouched: false          // the VIEWER's own vouch; never a count
    property bool   lookalikeWarning: false
    property int    markSize: DTheme.markInFeed

    spacing: 10

    Identicon {
        address: root.identityKey
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
        address: root.identityKey
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
