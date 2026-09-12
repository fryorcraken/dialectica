import QtQuick

// The contour-and-weave mark. Deterministic: the same address produces the
// same pixels on every peer. Six bytes of the address are used and the rest is
// not, which is exactly why this can never stand in for showing the address.
Canvas {
    id: root

    property string address: ""       // with or without a "stoa:" / "k:" prefix
    property bool   isPerson: false   // false = Stoa (angular), true = person (curved)
    property int    size: 40
    property int    stroke: size >= 34 ? 3 : 2

    width: size
    height: size
    onAddressChanged: requestPaint()
    onIsPersonChanged: requestPaint()
    onSizeChanged: requestPaint()

    readonly property var inks: [Theme.ink, Theme.accent, Theme.accent2]

    function _hex() {
        var a = address.replace(/^[a-z]+:/i, "").replace(/[^0-9a-f]/gi, "");
        return a.length >= 12 ? a : (a + "000000000000").slice(0, 12);
    }
    function _byte(i) { return parseInt(_hex().substr(i * 2, 2), 16); }

    // byte 1 picks the outline; bytes 2 and 3 pick an ORDERED pair for the
    // fill, always two different inks, both opaque.
    function _outlineInk() { return inks[_byte(1) % 3]; }
    function _fillInkA()   { return inks[_byte(2) % 3]; }
    function _fillInkB()   {
        var b = inks[_byte(3) % 3];
        return b === _fillInkA() ? inks[(_byte(3) + 1) % 3] : b;
    }
    function _angleDeg()   { return (_byte(4) % 24) * 15; }
    function _pitch()      { return 2 + (_byte(5) % 6); }   // 2..7 px
    function _sides()      { return 3 + (_byte(0) % 6); }   // 3..8
    function _cutCorner()  { return (_byte(0) % 7) === 6; }
    function _curvedForm() { return _byte(0) % 5; }         // 0..4

    function _roundRect(ctx, x, y, w, h, tl, tr, br, bl) {
        ctx.moveTo(x + tl, y);
        ctx.lineTo(x + w - tr, y);      ctx.quadraticCurveTo(x + w, y, x + w, y + tr);
        ctx.lineTo(x + w, y + h - br);  ctx.quadraticCurveTo(x + w, y + h, x + w - br, y + h);
        ctx.lineTo(x + bl, y + h);      ctx.quadraticCurveTo(x, y + h, x, y + h - bl);
        ctx.lineTo(x, y + tl);          ctx.quadraticCurveTo(x, y, x + tl, y);
        ctx.closePath();
    }

    function _tracePath(ctx, x, y, w, h) {
        ctx.beginPath();
        if (!isPerson) {
            if (_cutCorner()) {
                var c = w * 0.26;
                ctx.moveTo(x, y);
                ctx.lineTo(x + w, y);
                ctx.lineTo(x + w, y + h - c);
                ctx.lineTo(x + w - c, y + h);
                ctx.lineTo(x, y + h);
                ctx.closePath();
            } else {
                var n = _sides();
                var cx = x + w / 2, cy = y + h / 2, r = Math.min(w, h) / 2;
                for (var i = 0; i < n; i++) {
                    var a = -Math.PI / 2 + i * 2 * Math.PI / n;
                    var px = cx + r * Math.cos(a), py = cy + r * Math.sin(a);
                    if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
                }
                ctx.closePath();
            }
        } else {
            var rr = Math.min(w, h);
            switch (_curvedForm()) {
            case 0: ctx.ellipse(x, y, w, h); break;                                                  // circle
            case 1: _roundRect(ctx, x, y, w, h, rr * 0.28, rr * 0.28, rr * 0.28, rr * 0.28); break;  // rounded
            case 2: _roundRect(ctx, x, y, w, h, rr * 0.50, rr * 0.24, rr * 0.50, rr * 0.24); break;  // leaf
            case 3: _roundRect(ctx, x, y, w, h, rr * 0.50, rr * 0.50, rr * 0.24, rr * 0.50); break;  // teardrop
            default: _roundRect(ctx, x, y, w, h, h / 2, h / 2, h / 2, h / 2); break;                 // capsule
            }
        }
    }

    onPaint: {
        var ctx = getContext("2d");
        ctx.reset();
        if (size < Theme.markMinDraw)
            return;   // the caller prints the address instead

        // The outline is always drawn, so a mark never bleeds into the row behind it.
        _tracePath(ctx, 0, 0, size, size);
        ctx.fillStyle = _outlineInk();
        ctx.fill();

        var s = stroke;
        ctx.save();
        _tracePath(ctx, s, s, size - 2 * s, size - 2 * s);
        ctx.clip();

        ctx.fillStyle = _fillInkA();
        ctx.fillRect(0, 0, size, size);

        if (size >= Theme.markMinWeave) {   // below this the weave turns to mud
            var pitch = _pitch();
            ctx.translate(size / 2, size / 2);
            ctx.rotate(_angleDeg() * Math.PI / 180);
            ctx.fillStyle = _fillInkB();
            for (var o = -size; o < size; o += pitch * 2)
                ctx.fillRect(-size, o, size * 2, pitch);
        }
        ctx.restore();
    }
}
