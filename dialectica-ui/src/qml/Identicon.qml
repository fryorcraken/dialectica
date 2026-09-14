import QtQuick

// The contour-and-weave mark.
//
// DETERMINISM IS THE WHOLE CONTRACT, and the honest version of it is
// PATTERN-identity rather than pixel-identity. Every selector below is an
// integer index off the key hex, so the same key picks the same form,
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
// EIGHT BYTES of the public key are read — bytes 4..11, one per dimension — and
// TWENTY-FOUR are not, which is exactly why this can never stand in for showing
// the key. The mark is a recognition aid; the public key is the identity. A
// reader who needs to know WHO this is reads the key, always.
//
// Why eight bytes and not some wider window: the mark's OUTPUT is about 16 bits
// of perceptually distinct results, so eight bytes of input (64 bits) already
// exceeds what the rendering can express by a factor of 2^48. Reading more
// bytes would change nothing a reader could see — the input was never the
// binding constraint, the perceptual space is. Widening the read to look
// thorough would be the exact confusion this file's design note argues against.
// That reasoning is about the COUNT and is unaffected by which eight.
//
// WHY 4..11 SPECIFICALLY: it is a window no other channel touches. AddressLabel
// shows head 8, middle 8, tail 6 of the hex body — bytes 0..3, 14..17 and
// 29..31 — and the generated name reads bytes 18..23. Bytes 4..11 touch none of
// those. A byte the abbreviation displays is worse than merely shared: it is a
// byte an attacker can grind while reading their progress off the screen, and it
// tells the reader nothing the key has not already told them.
//
// This window was 12..19 and overlapped the middle group on {14, 15, 16, 17} —
// HALF of what the mark read was already on screen, so only {12, 13, 18, 19}
// reached the reader as new information. That was a weaker version of the
// criticism this design makes of the bundle's original mark: reduced rather than
// eliminated. Moving the mark rather than the abbreviation is what preserves the
// 8-8-6 shape and its vanity-defeating middle group, which is the property worth
// keeping.
//
// THE GENERATED NAME NOW READS BYTES 18..23 OF THIS SAME VALUE, AND THAT CHANGES
// THE ARGUMENT COMPLETELY.
//
// An earlier version of this comment said bytes 0..11 were "reserved for the
// generated-name scheme"; a later one replied, in capitals, that THE MECHANISM
// IT DESCRIBED DOES NOT EXIST. Both were right when written and both are now
// superseded. The retraction was correct about the design it described: while
// the name derived from H(NAME_PREFIX || public_key) and the mark read the
// ADDRESS — SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 || public_key) — they were two
// different digests, so byte 3 of one and byte 3 of the other were unrelated,
// there was no shared space to overlap in, and no allocation was required or
// possible.
//
// Issue #80 deletes the author address and removes NAME_PREFIX. The name, the
// mark and the abbreviation all read the SAME 32 bytes now, with no hash between
// the key and any of them, so THE ALLOCATION IS REAL AND IS LOAD-BEARING. Two
// channels reading one byte are two searches that partly coincide. Disjointness
// is what makes grinding for a lookalike name and grinding for a lookalike mark
// independent searches whose costs MULTIPLY rather than add — the conclusion the
// old domain-separation argument reached, now resting on byte allocation, which
// is the thing that argument said could never carry it.
//
//     channel                 key bytes
//     abbreviation head       0..3
//     mark (this file)        4..11
//     abbreviation middle     14..17
//     generated name          18..23
//     abbreviation tail       29..31
//
// Bytes 12..13 and 24..28 are read by no channel. They are UNALLOCATED, NOT
// RESERVED: nothing depends on their value, and no channel may be extended onto
// them without the `generated-names` spec changing.
//
// The gate is `tst_identicon.qml`, which measures all three channels by probing
// the components rather than by restating their arithmetic — see the note there
// about the computed version that was deleted for passing under a real defect.
Canvas {
    id: root

    // The 32-byte value to render, as hex, with or without a "stoa:" / "k:"
    // prefix.
    //
    // **Still named `address`, and that is deliberate rather than overlooked.**
    // For an AUTHOR this now carries the public key; for a STOA it carries the
    // Stoa address, which issue #80 keeps. Renaming the property is a change to
    // every call site — FeedScreen, PostHeader, the screens — and those are
    // `key-identity-sweep`'s, which is also what rewires the author call sites
    // to pass a key instead of an address. Renaming here and rewiring there
    // would split one rename across two pieces and leave the tree not compiling
    // in between.
    //
    // The arithmetic below does not care: it reads 32 bytes of hex. What the
    // byte allocation above is about is which bytes each channel reads OF THE
    // AUTHOR'S KEY, and that is unchanged by the name this property carries.
    property string address: ""
    property int    size: 40
    property int    stroke: size >= 34 ? 3 : 2

    // De-emphasis for a mark standing beside something already acted on — an
    // author in the moderated list, where the mark is there to be recognised
    // rather than attended to.
    //
    // IT IS AN OPACITY ON THE ROOT, AND DELIBERATELY NOT A DRAWING CHANGE.
    // Desaturating the inks inside `onPaint` would make what a mark looks like
    // depend on the CONTEXT it is drawn in, so two peers rendering the same
    // person in different lists would disagree — which is precisely what the
    // determinism contract above exists to forbid. Opacity cannot reach a
    // selector: `onPaint` is not re-entered, and every value
    // `tst_identicon.qml` pins is unchanged by construction rather than by
    // promise.
    //
    // The contour dims with the fill, which is intended. Holding the outline at
    // full strength would make a muted mark MORE conspicuous in silhouette than
    // an unmuted one — the opposite of recessive.
    property bool   muted: false

    width: size
    height: size
    opacity: muted ? DTheme.markMutedAlpha : 1
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
    // The values are frozen constants in DTheme, not tunable tokens.
    readonly property var inks: [
        DTheme.markInk, DTheme.markIndigo, DTheme.markRust, DTheme.markGreen,
        DTheme.markSteel, DTheme.markLavender, DTheme.markSage
    ]

    // ---- the value's bytes ----------------------------------------------
    // The hex body, prefix stripped, padded to 64 characters so a short or
    // malformed value still renders something stable rather than throwing.
    // Peer-supplied strings reach this component, so it must not assume a
    // well-formed 32-byte value.
    readonly property string _body: {
        var a = address.replace(/^[a-z]+:/i, "").replace(/[^0-9a-f]/gi, "").toLowerCase();
        return (a + "00000000000000000000000000000000000000000000000000000000000000000").slice(0, 64);
    }
    function _byte(i) { return parseInt(_body.substr(i * 2, 2), 16); }

    // ---- the dimensions -------------------------------------------------
    // Each reads its own byte. Sharing a byte between two dimensions couples
    // them: the bundle's original mark drove side-count, cut-corner and
    // curved-form all from byte 0, so two of those three were always dead.

    // Contour. One pooled family of eleven forms available to EVERY identity.
    // The angular/curved split the first draft used halved the vocabulary any
    // one identity could reach, and it was restating what position already
    // said: a Stoa's mark renders in the Stoa header, a person's beside their
    // name in a post. That distinction is now carried by POSITION ALONE — if
    // a mark is ever rendered somewhere the context does not disambiguate,
    // that placement must label it.
    function _form() { return _byte(4) % 11; }

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
    function _inkA() { return inks[_byte(6) % 7]; }
    function _inkB() {
        var i = _byte(6) % 7;
        var j = _byte(7) % 6;           // 0..5, so the offset is never 0 mod 7
        return inks[(i + 1 + j) % 7];   // never equal to A
    }
    function _outlineInk() {
        var i = _byte(6) % 7;
        var j = _byte(7) % 6;
        var b = (i + 1 + j) % 7;        // B's index
        // Walk forward from B by an offset that skips A, so all three differ.
        var k = _byte(5) % 5;           // 0..4
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
    function _angleDeg() { return (_byte(8) % 12) * 15; }

    // Pitch. Four values rather than six: at feed size the interior is about
    // 15px, so pitch 6 and 7 both render as "one bar across the mark" and
    // differ by a pixel of bar width. Values that a reader cannot separate are
    // not dimensions, they are noise with a parameter attached.
    function _pitch() { return [2, 3, 4, 6][_byte(9) % 4]; }

    // Duty cycle: what fraction of each period ink B covers. This is what
    // makes the ink pair genuinely ordered, and it is its own visual
    // dimension — a mark that is mostly A with thin B lines reads differently
    // from one that is mostly B with thin A lines, independently of WHICH two
    // inks they are.
    function _duty() { return [0.30, 0.45, 0.62][_byte(10) % 3]; }

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
    function _weave() { return _byte(11) % 3; }

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
            // `row` is declared OUTSIDE the for-init, and must stay there. A
            // comma-separated declaration list in a for-init —
            // `for (var ry = -size, row = 0; ...)`, which is what this was —
            // is mis-emitted by qmlformat 6.8.3: it drops the comma and writes
            // `for (var ry = -sizerow = 0; ...)`, which does not parse. CI's
            // `QML parses` step runs qmlformat over every file and fails on
            // exactly that, so the defect is in the FORMATTER's output, not in
            // this source — which parses clean on both 6.8.3 and 6.10.3, and
            // round-trips clean on 6.10.3 only. Since `var` is function-scoped
            // the two forms are equivalent, and this one is portable.
            var row = 0;
            for (var ry = -size; ry < size; ry += period) {
                // Every other row is offset by half a period, so the lattice
                // reads as a texture rather than as two crossed band families.
                var shift = (row % 2 === 0) ? 0 : period / 2;
                for (var rx = -size; rx < size; rx += period)
                    ctx.fillRect(rx + shift, ry, dot, dot);
                row++;
            }
            ctx.restore();
            break;
        }
    }

    onPaint: {
        var ctx = getContext("2d");
        ctx.reset();
        if (size < DTheme.markMinDraw)
            return;   // the caller prints the identifier instead

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

        if (size >= DTheme.markMinWeave)   // below this the weave turns to mud
            _paintWeave(ctx);

        ctx.restore();
    }
}
