import QtQuick

// The contour-and-weave mark.
//
// DETERMINISM IS THE WHOLE CONTRACT, and the honest version of it is
// PATTERN-identity rather than pixel-identity. Every selector below is an
// integer index off the address hex, so the same address picks the same form,
// the same three inks, the same weave, angle, pitch and duty on every peer
// forever. There is no randomness, no clock, no locale, no system font, and no
// float is accumulated across iterations or compared for equality.
//
// What is NOT promised is byte-identical rasterisation. Device pixel ratio
// changes the sample grid; at size 19 with stroke 2 the polygon radius is 7.5,
// putting axis vertices on exact half-integers where a fill rule decides the
// boundary pixel; and every weave rotates by a non-multiple of 90 degrees for 11
// of 12 angles, so edge coverage is an antialiasing detail. None of that can
// change WHICH shape or WHICH inks are drawn, which is the part recognition
// depends on and the part this contract covers.
//
// EIGHT BYTES of the address are read — bytes 12..19, one per dimension — and
// TWENTY-FOUR are not, which is exactly why this can never stand in for showing
// the address. The mark is a recognition aid; the address is the identity. A
// reader who needs to know WHO this is reads the address, always.
//
// Why 12..19 and not some wider window: the mark's OUTPUT is about 16 bits of
// perceptually distinct results, so eight bytes of input (64 bits) already
// exceeds what the rendering can express by a factor of 2^48. Reading more
// bytes would change nothing a reader could see — the input was never the
// binding constraint, the perceptual space is. Widening the read to look
// thorough would be the exact confusion this file's design note argues against.
//
// Bytes 0..11 are reserved for the generated-name scheme. That disjointness is
// load-bearing: grinding for a lookalike NAME and grinding for a lookalike MARK
// are then independent searches whose costs multiply rather than add.
//
// One honest limitation. AddressLabel abbreviates to head 8, middle 8, tail 6
// of the hex body — bytes 0..3, 14..17 and 29..31 — so bytes 14..17 are ALREADY
// on screen in the middle group. Half of what the mark reads therefore sits on
// ground the abbreviation covers, and only {12, 13, 18, 19} of the 21 bytes the
// abbreviation hides reach the reader through the mark. That is a weaker version
// of the criticism this design makes of the bundle's original, reduced rather
// than eliminated.
Canvas {
    id: root

    property string address: ""       // with or without a "stoa:" / "k:" prefix
    property int    size: 40
    property int    stroke: size >= 34 ? 3 : 2

    width: size
    height: size
    onAddressChanged: requestPaint()
    onSizeChanged: requestPaint()

    // Six inks, hand-picked rather than sliced off a hue wheel, and ordered as
    // a LIGHTNESS ladder because that is what survives colour-vision
    // deficiency: under deuteranopia and protanopia the red/green axis
    // collapses, so inks separated only by hue become one colour.
    //
    // Measured minimum pairwise OKLab distance is 0.147 in normal vision and
    // 0.108 under simulated dichromacy, with every ink also held above 0.20
    // contrast against the paper and clear of the fluorescence case (high chroma
    // at high lightness). Seven, because an eighth measured 0.045 against an
    // existing rung — see docs/IDENTICON.md.
    //
    // ORDER IS PART OF THE CONTRACT. These indices decide what every identity
    // looks like, so inserting, removing or reordering an entry changes every
    // mark in the system and makes two app versions disagree about the same
    // person. tst_identicon.qml pins the indexing for exactly this reason: a
    // rotation applied consistently across all three selectors is otherwise
    // invisible to a distinctness test.
    //
    // The values are frozen constants in Theme, not tunable tokens.
    readonly property var inks: [
        Theme.markInk, Theme.markIndigo, Theme.markRust, Theme.markGreen,
        Theme.markSteel, Theme.markLavender, Theme.markSage
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

    // Contour. One pooled family of eleven forms available to EVERY address.
    // The angular/curved split the first draft used halved the vocabulary any
    // one address could reach, and it was restating what position already
    // said: a Stoa's mark renders in the Stoa header, a person's beside their
    // name in a post. That distinction is now carried by POSITION ALONE — if
    // a mark is ever rendered somewhere the context does not disambiguate,
    // that placement must label it.
    function _form() { return _byte(12) % 11; }

    // Two inks for the weave and one for the outline, ALL THREE DISTINCT.
    //
    // The pair is ORDERED, so (rust, sky) and (sky, rust) are different marks.
    // Order survives rendering here because the duty cycle is not 50% — see
    // _duty(). At 50% a swap is a half-phase shift of an unanchored stripe
    // field, which is invisible, and the bundle's design claimed an ordering it
    // did not deliver.
    //
    // Each is derived by an OFFSET from the previous rather than an independent
    // draw, which is what guarantees distinctness by construction instead of by
    // a guard that has to be right at every call site. An earlier version drew
    // the outline independently, so one mark in seven had ring and ground the
    // same colour and no visible contour at all — the outline silently vanished
    // on ~14% of identities.
    function _inkA() { return inks[_byte(14) % 7]; }
    function _inkB() {
        var i = _byte(14) % 7;
        var j = _byte(15) % 6;          // 0..5, so the offset is never 0 mod 7
        return inks[(i + 1 + j) % 7];   // never equal to A
    }
    function _outlineInk() {
        var i = _byte(14) % 7;
        var j = _byte(15) % 6;
        var b = (i + 1 + j) % 7;        // B's index
        // Walk forward from B by an offset that skips A, so all three differ.
        var k = _byte(13) % 5;          // 0..4
        var c = b;
        for (var step = 0; step <= k; step++) {
            c = (c + 1) % 7;
            if (c === i) c = (c + 1) % 7;   // never A
        }
        return inks[c];
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
    // TOPOLOGY of the fill (parallel bands / crossed lattice / dot lattice)
    // rather than the orientation or spacing of one band family. A grinder who
    // matches a target's angle and pitch still has to match this, and no small
    // change to it looks like a small change.
    //
    // A CONCENTRIC-RING variant was tried here and removed. It failed the 19px
    // test in two different ways at once, which is why it looked like two
    // separate bugs: ring radii step by `period` (4..12px), so on a 19px mark
    // at most one band is ever visible, and whether the outermost disc lands
    // on ink A or ink B decides whether the mark reads as a BULLSEYE (a
    // different visual idiom from the rest of the family) or as a FLAT DISC
    // carrying no pattern at all. Both are a mark that has stopped
    // distinguishing anything. A pattern whose legibility depends on the
    // parity of a radius count is not a dimension.
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

    // Ten pooled forms, every one of which must have a silhouette a reader can
    // name at 19px. Two rules produced this list, and both are exclusions:
    //
    // NO CIRCLE. A circle is the absence of corners, which is exactly what
    // every polygon degrades toward as it shrinks — so it is at once the least
    // informative shape and the shape its neighbours collapse into at feed
    // size. It cost its neighbours distinctness, not only its own. (The
    // bundle's "capsule" was already a circle in disguise: a roundRect with
    // every corner at h/2 on a square box IS a circle, so that form is absent
    // for the same reason rather than as a separate judgement.)
    //
    // NO SIDE-COUNTS ABOVE 6. A heptagon and an octagon differ by about a
    // sixth of a pixel of silhouette at 19px. They would inflate the array
    // without widening what a reader can see, which is the defect this whole
    // change exists to repair.
    //
    // What remains all separates on a feature that survives shrinking: flat
    // ends, vertex orientation, concave points, or one cut corner.
    function _tracePath(ctx, x, y, w, h) {
        ctx.beginPath();
        var cx = x + w / 2, cy = y + h / 2;
        var r = Math.min(w, h) / 2;
        var rr = Math.min(w, h);
        var up = -Math.PI / 2;
        switch (_form()) {
        case 0:  _roundRect(ctx, x, y, w, h, rr * 0.28, rr * 0.28,
                            rr * 0.28, rr * 0.28); break;                          // rounded square
        case 1:  _roundRect(ctx, x, y, w, h, rr * 0.50, rr * 0.16,
                            rr * 0.50, rr * 0.16); break;                          // leaf
        case 2:  _roundRect(ctx, x, y, w, h, rr * 0.50, rr * 0.50,
                            rr * 0.10, rr * 0.50); break;                          // teardrop
        case 3:  _polygon(ctx, cx, cy, r, 3, up); break;                           // triangle
        case 4:  _polygon(ctx, cx, cy, r, 3, -up); break;                          // inverted triangle
        case 5:  _polygon(ctx, cx, cy, r, 4, up); break;                           // diamond
        case 6:  _polygon(ctx, cx, cy, r, 5, up); break;                           // pentagon
        case 7:  _polygon(ctx, cx, cy, r, 6, up); break;                           // hexagon
        case 8:  _star(ctx, cx, cy, r, 4, 0.42, up); break;                        // four-point star
        case 9:  _star(ctx, cx, cy, r, 6, 0.50, up); break;                        // six-point star
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
        default:                                  // dot lattice
            // A staggered grid of dots. Like the two band cases this covers
            // the WHOLE face at a constant density, so it cannot degenerate
            // into a flat fill the way the rings variant did — every cell of
            // the lattice carries the same amount of ink B regardless of where
            // the mark's centre falls.
            //
            // The dots are square rather than round because at 19px a radius-1
            // arc rasterises to an ambiguous smudge while a small fillRect keeps
            // a legible edge. This is a LEGIBILITY choice, not a determinism
            // one: the lattice is rotated by a non-multiple of 90 degrees for 11
            // of the 12 angles, so a rect's boundary coverage is as much an
            // antialiasing detail as an arc's would be.
            ctx.save();
            ctx.translate(size / 2, size / 2);
            ctx.rotate(_angleDeg() * Math.PI / 180);
            var dot = Math.max(1, Math.round(pitch * _duty() * 1.6));
            for (var ry = -size, row = 0; ry < size; ry += period, row++) {
                // Every other row is offset by half a period, so the lattice
                // reads as a texture rather than as two crossed band families.
                var shift = (row % 2 === 0) ? 0 : period / 2;
                for (var rx = -size; rx < size; rx += period)
                    ctx.fillRect(rx + shift, ry, dot, dot);
            }
            ctx.restore();
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
