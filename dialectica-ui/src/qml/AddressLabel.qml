import QtQuick

// The one place an address is abbreviated: head 8, middle 8, tail 6.
// Head-and-tail alone is the shape vanity-address generators are built to
// defeat; including the middle group makes a convincing near-match far more
// expensive. Full text is always what gets copied.
Text {
    id: root

    property string address: ""
    property bool   full: false       // true wherever a decision is being made
    property bool   emphasis: false   // e.g. a lookalike's key, in accent

    function abbreviate(a) {
        var m = a.match(/^([a-z]+:)?(.*)$/i);
        var prefix = m[1] || "";
        var body = m[2];
        var h = DTheme.headChars, mid = DTheme.middleChars, t = DTheme.tailChars;
        if (body.length <= h + mid + t + 4)
            return a;
        var start = Math.floor((body.length - mid) / 2);
        return prefix + body.substr(0, h) + "\u2026"
                      + body.substr(start, mid) + "\u2026"
                      + body.slice(-t);
    }

    property string copyText: address

    text: full ? address : abbreviate(address)
    font: full ? DTheme.addressBig : DTheme.address
    color: emphasis ? DTheme.accent : DTheme.inkMuted
    wrapMode: full ? Text.WrapAnywhere : Text.NoWrap
    textFormat: Text.PlainText

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.RightButton | Qt.LeftButton
        cursorShape: Qt.IBeamCursor
        onClicked: root.copyRequested()
    }
    signal copyRequested()
}
