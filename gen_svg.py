#!/usr/bin/env python3
"""Generate hex-terrain materials distribution demo SVGs.

Default (lean): white fills, no strokes — geometry-only for pipeline/Three.js.
  --rich:        random HSV colors, gradient blending, strokes, dots, comments.

Outputs:
  materials-distro-demo.svg       (always, lean)
  materials-distro-demo-rich.svg  (only with --rich)
"""
import argparse
import random
import math

SEED = 2026
S = 52   # cell spacing (point_spacing in HGridSettings)
R = 21   # visual hex radius
SQRT3 = math.sqrt(3)
W, HT = 680, 500
CX, CY = 340, 250


# ── Geometry helpers ──────────────────────────────────────────────

def hex_center(q, r):
    return (CX + S * 1.5 * q, CY + S * (SQRT3 / 2 * q + SQRT3 * r))

def hex_verts(cx, cy):
    return [(cx + R * math.cos(math.radians(60 * i)),
             cy + R * math.sin(math.radians(60 * i))) for i in range(6)]

def fmt(v):
    return f"{v[0]:.1f},{v[1]:.1f}"

def pts(vs):
    return " ".join(fmt(v) for v in vs)

def mid(a, b):
    return ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)


# ── Color helpers ─────────────────────────────────────────────────

def rand_hsv_color():
    h = random.random() * 360
    s = 0.65 + random.random() * 0.2
    v = 0.6 + random.random() * 0.25
    c = v * s
    x = c * (1 - abs((h / 60) % 2 - 1))
    m = v - c
    if h < 60:    rgb = (c, x, 0)
    elif h < 120: rgb = (x, c, 0)
    elif h < 180: rgb = (0, c, x)
    elif h < 240: rgb = (0, x, c)
    elif h < 300: rgb = (x, 0, c)
    else:         rgb = (c, 0, x)
    return "#{:02x}{:02x}{:02x}".format(*(int((ch + m) * 255) for ch in rgb))


# ── Grid + topology ──────────────────────────────────────────────

EDGE_DEFS = {
    0: ((1, 0),   (0, 1), (4, 3)),
    2: ((-1, 1),  (2, 3), (0, 5)),
    4: ((0, -1),  (4, 5), (2, 1)),
}
TRI_DEFS = {
    0: ((1, 0), (1, -1), 0, 4, 2),
    1: ((1, 0), (0, 1),  1, 3, 5),
}


def build_grid():
    """Build hex grid, assign random colors, compute quads and tris."""
    random.seed(SEED)

    hexes = sorted(
        (q, r) for q in range(-2, 3) for r in range(-2, 3)
        if max(abs(q), abs(r), abs(q + r)) <= 2
    )
    hex_set = set(hexes)
    colors = {h: rand_hsv_color() for h in hexes}
    centers = {h: hex_center(*h) for h in hexes}
    verts = {h: hex_verts(*centers[h]) for h in hexes}

    quads = []
    for hx in hexes:
        for ei, (d, (ov0, ov1), (nv0, nv1)) in EDGE_DEFS.items():
            nb = (hx[0] + d[0], hx[1] + d[1])
            if nb not in hex_set:
                continue
            quads.append(dict(
                hex=hx, edge=ei, neighbor=nb,
                points=[verts[hx][ov0], verts[nb][nv0],
                        verts[nb][nv1], verts[hx][ov1]],
                color_a=colors[hx], color_b=colors[nb],
                gfrom=mid(verts[hx][ov0], verts[hx][ov1]),
                gto=mid(verts[nb][nv0], verts[nb][nv1]),
            ))

    tris = []
    for hx in hexes:
        for vi, (d1, d2, hv, n1v, n2v) in TRI_DEFS.items():
            n1 = (hx[0] + d1[0], hx[1] + d1[1])
            n2 = (hx[0] + d2[0], hx[1] + d2[1])
            if n1 not in hex_set or n2 not in hex_set:
                continue
            tris.append(dict(
                hex=hx, vertex=vi, n1=n1, n2=n2,
                points=[verts[hx][hv], verts[n1][n1v], verts[n2][n2v]],
                colors=[colors[hx], colors[n1], colors[n2]],
                base_idx=random.randint(0, 2),
            ))

    return hexes, colors, centers, verts, quads, tris


# ── SVG header comment (rich only) ───────────────────────────────

RICH_HEADER = """\
  <!--
    ═══════════════════════════════════════════════════════════════════
    materials-distro-demo-rich.svg — hex-terrain gap-fill color blending
    ═══════════════════════════════════════════════════════════════════

    TRI COLOR BLENDING (triple-junction vertex, 3 HCells meet)
    ───────────────────────────────────────────────────────────────────
    Each Tri sits at a GridVertex where 3 HCells share a corner.

      vertex 0 → TriOwner        (HCell A)
      vertex 1 → TriPos1Emitter  (HCell B)
      vertex 2 → TriPos2Emitter  (HCell C)

    CENTER COLOR — picked at random from one of the 3 corners.
    The base HCell's color fills the entire triangle as solid background.
    The center of the tri — where no overlay gradient reaches — shows
    that base color pure.

    CORNER OVERLAYS — the other 2 corners paint over the base:
      Each non-base corner gets a linearGradient:
        SOLID (opacity=1) at corner vertex              0% – 42.5%
        BLEND HOTSPOT: 15% transition zone             42.5% – 57.5%
        TRANSPARENT (opacity=0) at opposite edge       57.5% – 100%
      Rendered at opacity="0.9".

    Result: each corner shows its HCell color, center shows the random
    base color, edges between corners have a narrow 15% blend hotspot.

    QUAD COLOR BLENDING (even-edge gap between 2 HCells)
    ───────────────────────────────────────────────────────────────────
    Simpler: single linearGradient perpendicular to the shared edge.
      Owner side solid     0% – 42.5%   |  15% blend  |  Neighbor side solid  57.5% – 100%

    Entity counts: {nhex} HCells | {nquad} Quads | {ntri} Tris
    ═══════════════════════════════════════════════════════════════════
  -->"""


# ── SVG generators ────────────────────────────────────────────────

def generate_lean(hexes, verts, quads, tris):
    """White fills, no strokes, no dots — geometry only."""
    L = []
    a = L.append
    a(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {HT}" width="{W}" height="{HT}">')
    a(f'  <rect width="{W}" height="{HT}" rx="8" fill="#050810"/>')

    # All shapes: white fill, no stroke
    for q in quads:
        a(f'  <polygon points="{pts(q["points"])}" fill="#fff"/>')
    for tri in tris:
        a(f'  <polygon points="{pts(tri["points"])}" fill="#fff"/>')
    for hx in hexes:
        a(f'  <polygon id="hcell-q{hx[0]}r{hx[1]}" points="{pts(verts[hx])}" fill="#fff"/>')

    a('</svg>')
    return "\n".join(L)


def generate_rich(hexes, colors, verts, quads, tris):
    """Random HSV colors, gradient blending, strokes, dots, comments."""
    L = []
    a = L.append
    a(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {HT}" width="{W}" height="{HT}">')
    a(RICH_HEADER.format(nhex=len(hexes), nquad=len(quads), ntri=len(tris)))
    a('  <defs>')

    # Quad gradients
    for q in quads:
        h, e = q['hex'], q['edge']
        gid = f"gQ-e{e}-q{h[0]}r{h[1]}"
        f, t = q['gfrom'], q['gto']
        ca, cb = q['color_a'], q['color_b']
        a(f'    <linearGradient id="{gid}" gradientUnits="userSpaceOnUse"')
        a(f'        x1="{f[0]:.1f}" y1="{f[1]:.1f}" x2="{t[0]:.1f}" y2="{t[1]:.1f}">')
        a(f'      <stop offset="0%"    stop-color="{ca}"/>')
        a(f'      <stop offset="42.5%" stop-color="{ca}"/>')
        a(f'      <stop offset="57.5%" stop-color="{cb}"/>')
        a(f'      <stop offset="100%"  stop-color="{cb}"/>')
        a(f'    </linearGradient>')

    # Tri gradients (2 overlays per tri for non-base corners)
    for tri in tris:
        h, vi, bi = tri['hex'], tri['vertex'], tri['base_idx']
        for j in range(3):
            if j == bi:
                continue
            corner = tri['points'][j]
            opp = mid(*[tri['points'][k] for k in range(3) if k != j])
            col = tri['colors'][j]
            gid = f"gT{j}-v{vi}-q{h[0]}r{h[1]}"
            a(f'    <linearGradient id="{gid}" gradientUnits="userSpaceOnUse"')
            a(f'        x1="{corner[0]:.1f}" y1="{corner[1]:.1f}" x2="{opp[0]:.1f}" y2="{opp[1]:.1f}">')
            a(f'      <stop offset="0%"    stop-color="{col}" stop-opacity="1"/>')
            a(f'      <stop offset="42.5%" stop-color="{col}" stop-opacity="1"/>')
            a(f'      <stop offset="57.5%" stop-color="{col}" stop-opacity="0"/>')
            a(f'      <stop offset="100%"  stop-color="{col}" stop-opacity="0"/>')
            a(f'    </linearGradient>')

    a('  </defs>')
    a(f'  <rect width="{W}" height="{HT}" rx="8" fill="#050810"/>')
    a('')

    STROKE = ' stroke="#fff" stroke-opacity="0.5" stroke-width="1.5" stroke-linejoin="round"'

    # Quads
    a(f'  <!-- {len(quads)} Quad gap-fillers (even edges [0,2,4]) -->')
    for q in quads:
        h, e, nb = q['hex'], q['edge'], q['neighbor']
        gid = f"gQ-e{e}-q{h[0]}r{h[1]}"
        p = pts(q['points'])
        a(f'  <!-- QuadOwner=HCell({h[0]},{h[1]}) edge{e} → HCell({nb[0]},{nb[1]}) -->')
        a(f'  <polygon id="quad-e{e}-q{h[0]}r{h[1]}" points="{p}"')
        a(f'           fill="url(#{gid})"{STROKE}/>')
    a('')

    # Tris
    a(f'  <!-- {len(tris)} Tri gap-fillers (triple-junction vertices) -->')
    for tri in tris:
        h, vi = tri['hex'], tri['vertex']
        n1, n2, bi = tri['n1'], tri['n2'], tri['base_idx']
        p = pts(tri['points'])
        base_color = tri['colors'][bi]
        a(f'  <!-- TriOwner=HCell({h[0]},{h[1]}) v{vi} base=corner{bi} -->')
        a(f'  <polygon id="tri-v{vi}-q{h[0]}r{h[1]}" points="{p}" fill="{base_color}"/>')
        for j in range(3):
            if j == bi:
                continue
            gid = f"gT{j}-v{vi}-q{h[0]}r{h[1]}"
            a(f'  <polygon points="{p}" fill="url(#{gid})" opacity="0.9"/>')
        a(f'  <polygon points="{p}" fill="none"{STROKE}/>')
    a('')

    # Hex faces
    a(f'  <!-- {len(hexes)} HCell hex faces -->')
    for hx in hexes:
        color = colors[hx]
        p = pts(verts[hx])
        a(f'  <polygon id="hcell-q{hx[0]}r{hx[1]}" points="{p}"')
        a(f'           fill="{color}" fill-opacity="0.85"{STROKE}/>')
    a('')

    # Junction dots
    a('  <!-- Corner entities at triple-junction GridVertex positions -->')
    a('  <g fill="#fff" opacity="0.4">')
    seen = set()
    for tri in tris:
        for pt in tri['points']:
            key = (round(pt[0], 1), round(pt[1], 1))
            if key not in seen:
                seen.add(key)
                a(f'    <circle cx="{key[0]}" cy="{key[1]}" r="1.2"/>')
    a('  </g>')

    a('</svg>')
    return "\n".join(L)


# ── Main ──────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--rich', action='store_true',
                        help='Also generate the rich (colored, stroked) variant')
    args = parser.parse_args()

    hexes, colors, centers, verts, quads, tris = build_grid()

    lean = generate_lean(hexes, verts, quads, tris)
    with open("materials-distro-demo.svg", "w") as f:
        f.write(lean)
    print(f"  materials-distro-demo.svg       (lean: {len(lean.splitlines())} lines)")

    if args.rich:
        rich = generate_rich(hexes, colors, verts, quads, tris)
        with open("materials-distro-demo-rich.svg", "w") as f:
            f.write(rich)
        print(f"  materials-distro-demo-rich.svg  (rich: {len(rich.splitlines())} lines)")

    print(f"  {len(hexes)} HCells | {len(quads)} Quads | {len(tris)} Tris")


if __name__ == "__main__":
    main()
