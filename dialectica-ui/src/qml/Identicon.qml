import QtQuick

// The contour-and-weave mark.
//
// DETERMINISM IS THE WHOLE CONTRACT. The same address must produce the same
// pixels on every peer, forever. Everything below is integer-indexed off the
// address hex; there is no randomness, no clock, no locale, no system font,
// and no floating-point value is ever accumulated across iterations or
// compared for equality. The only floating-point arithmetic is the per-shape
// trigonometry the Canvas would do anyway, computed fresh from integers each
// time.
//
// TWENTY BYTES of the address are read (bytes 12..31) and TWELVE are not,
// which is exactly why this can never stand in for showing the address. The
// mark is a recognition aid; the address is the identity. A reader who needs
// to know WHO this is reads the address, always.
//
// The byte range is chosen, not arbitrary. AddressLabel abbreviates to head 8,
// middle 8, tail 6 of the hex body — that is bytes 0..3, 14..17 and 29..31, so
// 21 of the 32 bytes are invisible at feed density. Reading the high range
// puts most of that hidden material on screen in a form a reader could notice.
// Bytes 0..11 are left to the generated-name scheme, so that grinding for a
// lookalike NAME and grinding for a lookalike MARK are independent searches
// whose costs multiply rather than add.
Canvas {
    id: root

    property string address: ""       // with or without a "stoa:" / "k:" prefix
    property int    size: 40
    property int    stroke: size >= 34 ? 3 : 2

    width: size
    height: size
    onAddressChanged: requestPaint()
    onSizeChanged: requestPaint()

    // Eight inks, hand-picked rather than sliced off a hue wheel. Adjacent
    // steps on a wheel land inside a just-noticeable difference and produce
    // parameter states nobody can tell apart; these are separated in
    // LIGHTNESS and CHROMA as well as hue, which is what lets a categorical
    // palette exceed what hue rotation alone can distinguish.
    //
    // Minimum pairwise OKLab distance is 0.080 in normal vision. Under
    // simulated deuteranopia and protanopia — where the red/green axis
    // collapses — every pair still separates on lightness or on the
    // blue/yellow axis, which is why Moss is dark and Ochre is light rather
    // than both sitting mid-range.
    readonly property var inks: [
        Theme.markInk, Theme.markIndigo, Theme.markMoss, Theme.markPlum,
        Theme.markRust, Theme.markTeal, Theme.markStone, Theme.markOchre
    ]

    // ---- address bytes --------------------------------------------------
    // The hex body, prefix stripped, padded to 64 characters so a short or
    // malformed address still renders something stable rather than throwing.
    // Peer-supplied strings reach this component, so it must not assume a
    // well-formed address.
    readonly property string _body: {
        var a = address.replace(/^[a-z]+:/i, "").replace(/[^0-9a-f]/gi, "").toLowerCase();
        return (a + "00000000000000000000000000000000000000000000000000000000000000000").slice(0, 64);
    }
    function _byte(i) { return parseInt(_body.substr(i * 2, 2), 16); }

    // ---- the dimensions -------------------------------------------------
    // Each reads its own byte. Sharing a byte between two dimensions couples
    // them: the bundle's original mark drove side-count, cut-corner and
    // curved-form all from byte 0, so two of those three were always dead.

    // Contour. One pooled family of twelve forms available to EVERY address.
    // The angular/curved split the first draft used halved the vocabulary any
    // one address could reach, and it was restating what position already
    // said: a Stoa's mark renders in the Stoa header, a person's beside their
    // name in a post. That distinction is now carried by POSITION ALONE — if
    // a mark is ever rendered somewhere the context does not disambiguate,
    // that placement must label it.
    function _form() { return _byte(12) % 12; }

    // Two inks for the weave and one for the outline. The pair is ORDERED and
    // the two are always different, so (rust, teal) and (teal, rust) are
    // different marks. Order survives rendering here because the duty cycle is
    // not 50% — see _duty(). At 50% a swap is a half-phase shift of an
    // unanchored stripe field, which is invisible, and the original design
    // claimed an ordering it did not deliver.
    function _outlineInk() { return inks[_byte(13) % 8]; }
    function _inkA()       { return inks[_byte(14) % 8]; }
    function _inkB() {
        var i = _byte(14) % 8;
        var j = _byte(15) % 7;          // 0..6, an offset that is never 0 mod 8
        return inks[(i + 1 + j) % 8];   // always a different ink from A
    }

    // Weave angle. Parallel stripes have period 180 degrees, not 360 — a field
    // at 15 and at 195 degrees is the same field — so the original 24 steps of
    // 15 degrees were really 12. Twelve steps of 15 degrees over a half turn
    // is the honest version of the same dimension.
    function _angleDeg() { return (_byte(16) % 12) * 15; }

    // Pitch. Four values rather than six: at feed size the interior is about
    // 15px, so pitch 6 and 7 both render as "one bar across the mark" and
    // differ by a pixel of bar width. Values that a reader cannot separate are
    // not dimensions, they are noise with a parameter attached.
    function _pitch() { return [2, 3, 4, 6][_byte(17) % 4]; }

    // Duty cycle: what fraction of each period ink B covers. This is what
    // makes the ink pair genuinely ordered, and it is its own visual
    // dimension — a mark that is mostly A with thin B lines reads differently
    // from one that is mostly B with thin A lines, independently of WHICH two
    // inks they are.
    function _duty() { return [0.30, 0.45, 0.62][_byte(18) % 3]; }

    // Weave kind. Perceptually independent of angle and pitch: it changes the
    // TOPOLOGY of the fill (parallel bands / crossed lattice / concentric
    // rings) rather than the orientation or spacing of one band family. A
    // grinder who matches a target's angle and pitch still has to match this,
    // and no small change to it looks like a small change.
    function _weave() { return _byte(19) % 3; }

    // ---- contours ---------------------------------------------------------
    function _roundRect(ctx, x, y, w, h, tl, tr, br, bl) {
        ctx.moveTo(x + tl, y);
        ctx.lineTo(x + w - tr, y);      ctx.quadraticCurveTo(x + w, y, x + w, y + tr);
        ctx.lineTo(x + w, y + h - br);  ctx.quadraticCurveTo(x + w, y + h, x + w - br, y + h);
        ctx.lineTo(x + bl, y + h);      ctx.quadraticCurveTo(x, y + h, x, y + h - bl);
        ctx.lineTo(x, y + tl);          ctx.quadraticCurveTo(x, y, x + tl, y);
        ctx.closePath();
    }

    function _polygon(ctx, cx, cy, r, n, rot) {
        for (var i = 0; i < n; i++) {
            var a = rot + i * 2 * Math.PI / n;
            var px = cx + r * Math.cos(a), py = cy + r * Math.sin(a);
            if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
        }
        ctx.closePath();
    }

    // A star: n outer points, n inner points at `inner` of the radius. Chosen
    // over adding more polygon side-counts because a 7-gon and an 8-gon are
    // one shape to the eye at 19px, whereas a star is never mistaken for a
    // polygon at any size that draws at all.
    function _star(ctx, cx, cy, r, n, inner, rot) {
        for (var i = 0; i < 2 * n; i++) {
            var rr = (i % 2 === 0) ? r : r * inner;
            var a = rot + i * Math.PI / n;
            var px = cx + rr * Math.cos(a), py = cy + rr * Math.sin(a);
            if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
        }
        ctx.closePath();
    }

    // Twelve pooled forms. Chosen so that each is distinguishable from every
    // other at 19px — the feed size, which is where recognition actually
    // happens. Side-counts above 6 are deliberately absent: a heptagon and an
    // octagon differ by about a sixth of a pixel of silhouette at feed size,
    // so they would inflate the array without widening what a reader can see.
    function _tracePath(ctx, x, y, w, h) {
        ctx.beginPath();
        var cx = x + w / 2, cy = y + h / 2;
        var r = Math.min(w, h) / 2;
        var rr = Math.min(w, h);
        var up = -Math.PI / 2;
        switch (_form()) {
        case 0:  ctx.ellipse(x, y, w, h); break;                                   // circle
        case 1:  _roundRect(ctx, x, y, w, h, rr * 0.28, rr * 0.28,
                            rr * 0.28, rr * 0.28); break;                          // rounded square
        case 2:  _roundRect(ctx, x, y, w, h, rr * 0.50, rr * 0.20,
                            rr * 0.50, rr * 0.20); break;                          // leaf
        case 3:  _roundRect(ctx, x, y, w, h, rr * 0.50, rr * 0.50,
                            rr * 0.18, rr * 0.50); break;                          // teardrop
        case 4:  _polygon(ctx, cx, cy, r, 3, up); break;                           // triangle
        case 5:  _polygon(ctx, cx, cy, r, 3, -up); break;                          // inverted triangle
        case 6:  _polygon(ctx, cx, cy, r, 4, up); break;                           // diamond
        case 7:  _polygon(ctx, cx, cy, r, 5, up); break;                           // pentagon
        case 8:  _polygon(ctx, cx, cy, r, 6, up); break;                           // hexagon
        case 9:  _star(ctx, cx, cy, r, 4, 0.46, up); break;                        // four-point star
        case 10: _star(ctx, cx, cy, r, 6, 0.55, up); break;                        // six-point star
        default:                                                                   // cut corner
            var c = w * 0.30;
            ctx.moveTo(x, y);
            ctx.lineTo(x + w, y);
            ctx.lineTo(x + w, y + h - c);
            ctx.lineTo(x + w - c, y + h);
            ctx.lineTo(x, y + h);
            ctx.closePath();
            break;
        }
    }

    // ---- the weave --------------------------------------------------------
    // Each kind fills the already-clipped interior with ink B over a solid
    // ink A ground. All three are drawn from integer loop counters so that no
    // rounding difference can accumulate differently on two machines.
    function _paintWeave(ctx) {
        var pitch = _pitch();
        var period = pitch * 2;
        var bar = Math.max(1, Math.round(period * _duty()));
        var span = size * 2;

        ctx.fillStyle = _inkB();
        switch (_weave()) {
        case 0:                                   // parallel bands
            ctx.save();
            ctx.translate(size / 2, size / 2);
            ctx.rotate(_angleDeg() * Math.PI / 180);
            for (var o = -size; o < size; o += period)
                ctx.fillRect(-size, o, span, bar);
            ctx.restore();
            break;
        case 1:                                   // crossed lattice
            ctx.save();
            ctx.translate(size / 2, size / 2);
            ctx.rotate(_angleDeg() * Math.PI / 180);
            for (var p = -size; p < size; p += period)
                ctx.fillRect(-size, p, span, bar);
            ctx.rotate(Math.PI / 2);
            for (var q = -size; q < size; q += period)
                ctx.fillRect(-size, q, span, bar);
            ctx.restore();
            break;
        default:                                  // concentric rings
            // Rings are anchored at the centre rather than being an unanchored
            // periodic field, so unlike stripes they carry no invisible phase.
            // The angle is spent instead on an offset of the ring centre,
            // which IS visible: it makes the rings eccentric.
            var a = _angleDeg() * Math.PI / 180;
            var ox = size / 2 + Math.cos(a) * size * 0.12;
            var oy = size / 2 + Math.sin(a) * size * 0.12;
            for (var k = Math.ceil(size / period); k >= 0; k--) {
                var rad = k * period;
                ctx.beginPath();
                ctx.arc(ox, oy, rad, 0, 2 * Math.PI);
                ctx.fillStyle = (k % 2 === 0) ? _inkB() : _inkA();
                ctx.fill();
            }
            break;
        }
    }

    onPaint: {
        var ctx = getContext("2d");
        ctx.reset();
        if (size < Theme.markMinDraw)
            return;   // the caller prints the address instead

        // The outline is always drawn, so a mark never bleeds into the row
        // behind it and is never transparent.
        _tracePath(ctx, 0, 0, size, size);
        ctx.fillStyle = _outlineInk();
        ctx.fill();

        var s = stroke;
        ctx.save();
        _tracePath(ctx, s, s, size - 2 * s, size - 2 * s);
        ctx.clip();

        ctx.fillStyle = _inkA();
        ctx.fillRect(0, 0, size, size);

        if (size >= Theme.markMinWeave)   // below this the weave turns to mud
            _paintWeave(ctx);

        ctx.restore();
    }
}
