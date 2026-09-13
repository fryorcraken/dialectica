import QtQuick
import QtQuick.Layouts

// The card: content on paper, apparatus column on the right. Every screen uses
// it, so a caveat always has somewhere to live next to the thing it qualifies.
Rectangle {
    id: root

    default property alias content: body.data
    property alias apparatus: app.content

    implicitWidth: DTheme.cardWidth
    color: DTheme.paper
    border.width: DTheme.hairline
    border.color: DTheme.ink

    RowLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            id: body
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            Layout.margins: DTheme.cardPaddingY
            Layout.leftMargin: DTheme.cardPaddingX
            Layout.rightMargin: DTheme.cardPaddingX
            spacing: DTheme.blockGap
        }

        ApparatusColumn {
            id: app
            Layout.fillHeight: true
            Layout.preferredWidth: DTheme.apparatusWidth
        }
    }
}
