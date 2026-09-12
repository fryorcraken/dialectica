import QtQuick
import QtQuick.Layouts

// The card: content on paper, apparatus column on the right. Every screen uses
// it, so a caveat always has somewhere to live next to the thing it qualifies.
Rectangle {
    id: root

    default property alias content: body.data
    property alias apparatus: app.content

    implicitWidth: Theme.cardWidth
    color: Theme.paper
    border.width: Theme.hairline
    border.color: Theme.ink

    RowLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            id: body
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            Layout.margins: Theme.cardPaddingY
            Layout.leftMargin: Theme.cardPaddingX
            Layout.rightMargin: Theme.cardPaddingX
            spacing: Theme.blockGap
        }

        ApparatusColumn {
            id: app
            Layout.fillHeight: true
            Layout.preferredWidth: Theme.apparatusWidth
        }
    }
}
