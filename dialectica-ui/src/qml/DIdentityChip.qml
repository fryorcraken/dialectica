import QtQuick
import QtQuick.Layouts

// The footer chip that says who you are in this Stoa — or that you are nobody
// here yet. It lives centred in the screen footer on every feed, so the answer
// to "who am I posting as" is always in the same place rather than somewhere
// that depends on what the screen is doing.
//
// WHY IT LABELS THE MARK. `Identicon` no longer distinguishes a person from a
// Stoa by contour: that split halved the vocabulary any one address could reach
// and restated what position already said, so the distinction is carried by
// POSITION ALONE (see Identicon.qml). The obligation attached to that decision
// is that a placement the context does not disambiguate must label itself — and
// a footer chip is such a placement, since a reader arriving at it cold has no
// surrounding post to say whose mark this is. `CURRENT IDENTITY` is that label.
// It is not decoration and should not be dropped to save a row.
//
// The address is on screen and not one click away, which is the standing rule
// wherever an identity is named: the mark is a recognition aid and a second
// forgeable channel, never an identifier.
Rectangle {
    id: root

    property bool hasIdentity: true
    property string generatedName: ""
    property string identityAddress: ""
    signal createRequested()

    implicitWidth:  row.implicitWidth + 24
    implicitHeight: row.implicitHeight + 8
    color: DTheme.field
    border.width: DTheme.hairline

    // The accent border when there is no identity is the chip asking for
    // attention: it is the one state in which the footer wants to be read.
    border.color: hasIdentity ? DTheme.rule2 : DTheme.accent

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: 9

        // ---- identity present -------------------------------------------
        // A RowLayout omits a `visible: false` child from its layout entirely,
        // so the two sets below never reserve space for each other.
        Identicon {
            visible: root.hasIdentity
            address: root.identityAddress
            size: 17
        }
        Text {
            visible: root.hasIdentity
            text: "CURRENT IDENTITY"          // copy.json common.currentIdentity
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }
        Text {
            visible: root.hasIdentity

            // A generated name is derived from the key and is NOT an
            // identifier — two keys can produce names that look alike, and
            // arrival order differs per peer so they are never numbered apart.
            // PlainText because a name is peer-supplied text like any other.
            text: root.generatedName
            font: DTheme.bodySmall
            color: DTheme.ink
            textFormat: Text.PlainText
        }
        AddressLabel {
            visible: root.hasIdentity
            address: root.identityAddress
        }

        // ---- no identity on this machine ---------------------------------
        Text {
            visible: !root.hasIdentity

            // copy.json feed.readOnlyFooter, verbatim. It names what an
            // identity is FOR rather than asserting a restriction: reading
            // needs nothing, and the sentence says so by omission.
            text: "Voting, posting and replying need an identity."
            font: DTheme.bodySmall
            color: DTheme.ink
            textFormat: Text.PlainText
        }
        FlatButton {
            visible: !root.hasIdentity
            text: "Create an identity"        // copy.json common.createIdentity
            kind: "primary"
            onClicked: root.createRequested()
        }
    }
}
