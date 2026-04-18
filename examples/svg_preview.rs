//! Generate hex-terrain materials distribution demo SVGs.
//!
//! Default (lean): white fills, no strokes — geometry-only for pipeline / Three.js.
//! `--rich`: random HSV colors, gradient blending, strokes, dots, XML comments.
//!
//! ```
//! cargo run --example svg_preview                # lean only
//! cargo run --example svg_preview -- --rich      # lean + rich
//! ```

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::fmt::Write as FmtWrite;
use std::fs;

const SEED: u64 = 2026;
const GRID_RADIUS: i32 = 2;
const CELL_SPACING: f64 = 52.0;
const HEX_RADIUS: f64 = 21.0;
const SQRT3: f64 = 1.732_050_808;
const W: f64 = 680.0;
const H: f64 = 500.0;
const CX: f64 = 340.0;
const CY: f64 = 250.0;

type Pt = (f64, f64);

// ── Geometry ─────────────────────────────────────────────────────

fn hex_center(q: i32, r: i32) -> Pt {
    (
        CX + CELL_SPACING * 1.5 * q as f64,
        CY + CELL_SPACING * (SQRT3 / 2.0 * q as f64 + SQRT3 * r as f64),
    )
}

fn hex_verts(cx: f64, cy: f64) -> [Pt; 6] {
    std::array::from_fn(|i| {
        let angle = PI / 3.0 * i as f64;
        (cx + HEX_RADIUS * angle.cos(), cy + HEX_RADIUS * angle.sin())
    })
}

fn fmt_pt(p: Pt) -> String {
    format!("{:.1},{:.1}", p.0, p.1)
}

fn pts(vs: &[Pt]) -> String {
    vs.iter().map(|v| fmt_pt(*v)).collect::<Vec<_>>().join(" ")
}

fn mid(a: Pt, b: Pt) -> Pt {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

fn in_grid(q: i32, r: i32) -> bool {
    q.abs().max(r.abs()).max((q + r).abs()) <= GRID_RADIUS
}

// ── Simple RNG (xorshift64) ─────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn float(&mut self) -> f64 {
        (self.next() % 10000) as f64 / 10000.0
    }
    fn int(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo + 1) as u64) as i32
    }
}

// ── Color ────────────────────────────────────────────────────────

fn hsv_to_hex(h: f64, s: f64, v: f64) -> String {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

fn rand_color(rng: &mut Rng) -> String {
    let h = rng.float() * 360.0;
    let s = 0.65 + rng.float() * 0.2;
    let v = 0.6 + rng.float() * 0.25;
    hsv_to_hex(h, s, v)
}

// ── Edge / Tri topology ──────────────────────────────────────────

struct EdgeDef {
    dq: i32,
    dr: i32,
    ov: (usize, usize),
    nv: (usize, usize),
}

const EDGE_DEFS: [(u8, EdgeDef); 3] = [
    (0, EdgeDef { dq: 1, dr: 0, ov: (0, 1), nv: (4, 3) }),
    (2, EdgeDef { dq: -1, dr: 1, ov: (2, 3), nv: (0, 5) }),
    (4, EdgeDef { dq: 0, dr: -1, ov: (4, 5), nv: (2, 1) }),
];

struct TriDef {
    d1: (i32, i32),
    d2: (i32, i32),
    hv: usize,
    n1v: usize,
    n2v: usize,
}

const TRI_DEFS: [(u8, TriDef); 2] = [
    (0, TriDef { d1: (1, 0), d2: (1, -1), hv: 0, n1v: 4, n2v: 2 }),
    (1, TriDef { d1: (1, 0), d2: (0, 1), hv: 1, n1v: 3, n2v: 5 }),
];

// ── Data structures ──────────────────────────────────────────────

struct Quad {
    hex: (i32, i32),
    edge: u8,
    neighbor: (i32, i32),
    points: [Pt; 4],
    color_a: String,
    color_b: String,
    grad_from: Pt,
    grad_to: Pt,
}

struct Tri {
    hex: (i32, i32),
    vertex: u8,
    n1: (i32, i32),
    n2: (i32, i32),
    points: [Pt; 3],
    colors: [String; 3],
    base_idx: usize,
}

// ── Grid builder ─────────────────────────────────────────────────

struct Grid {
    hexes: Vec<(i32, i32)>,
    colors: HashMap<(i32, i32), String>,
    verts: HashMap<(i32, i32), [Pt; 6]>,
    quads: Vec<Quad>,
    tris: Vec<Tri>,
}

fn build_grid() -> Grid {
    let mut rng = Rng::new(SEED);

    let mut hexes: Vec<(i32, i32)> = Vec::new();
    for q in -GRID_RADIUS..=GRID_RADIUS {
        for r in -GRID_RADIUS..=GRID_RADIUS {
            if in_grid(q, r) {
                hexes.push((q, r));
            }
        }
    }
    hexes.sort();

    let hex_set: HashSet<(i32, i32)> = hexes.iter().copied().collect();
    let colors: HashMap<_, _> = hexes.iter().map(|&h| (h, rand_color(&mut rng))).collect();
    let centers: HashMap<_, _> = hexes.iter().map(|&(q, r)| ((q, r), hex_center(q, r))).collect();
    let verts: HashMap<_, _> = hexes
        .iter()
        .map(|&h| {
            let (cx, cy) = centers[&h];
            (h, hex_verts(cx, cy))
        })
        .collect();

    let mut quads = Vec::new();
    for &hx in &hexes {
        for &(ei, ref ed) in &EDGE_DEFS {
            let nb = (hx.0 + ed.dq, hx.1 + ed.dr);
            if !hex_set.contains(&nb) {
                continue;
            }
            let hv = &verts[&hx];
            let nv = &verts[&nb];
            quads.push(Quad {
                hex: hx,
                edge: ei,
                neighbor: nb,
                points: [hv[ed.ov.0], nv[ed.nv.0], nv[ed.nv.1], hv[ed.ov.1]],
                color_a: colors[&hx].clone(),
                color_b: colors[&nb].clone(),
                grad_from: mid(hv[ed.ov.0], hv[ed.ov.1]),
                grad_to: mid(nv[ed.nv.0], nv[ed.nv.1]),
            });
        }
    }

    let mut tris = Vec::new();
    for &hx in &hexes {
        for &(vi, ref td) in &TRI_DEFS {
            let n1 = (hx.0 + td.d1.0, hx.1 + td.d1.1);
            let n2 = (hx.0 + td.d2.0, hx.1 + td.d2.1);
            if !hex_set.contains(&n1) || !hex_set.contains(&n2) {
                continue;
            }
            let base_idx = rng.int(0, 2) as usize;
            tris.push(Tri {
                hex: hx,
                vertex: vi,
                n1,
                n2,
                points: [verts[&hx][td.hv], verts[&n1][td.n1v], verts[&n2][td.n2v]],
                colors: [
                    colors[&hx].clone(),
                    colors[&n1].clone(),
                    colors[&n2].clone(),
                ],
                base_idx,
            });
        }
    }

    Grid { hexes, colors, verts, quads, tris }
}

// ── SVG generators ───────────────────────────────────────────────

fn generate_lean(g: &Grid) -> String {
    let mut s = String::with_capacity(8192);
    writeln!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">"#).unwrap();
    writeln!(s, r#"  <rect width="{W}" height="{H}" rx="8" fill="#050810"/>"#).unwrap();

    for q in &g.quads {
        writeln!(s, r#"  <polygon points="{}" fill="#fff"/>"#, pts(&q.points)).unwrap();
    }
    for tri in &g.tris {
        writeln!(s, r#"  <polygon points="{}" fill="#fff"/>"#, pts(&tri.points)).unwrap();
    }
    for &hx in &g.hexes {
        writeln!(
            s,
            r#"  <polygon id="hcell-q{}r{}" points="{}" fill="#fff"/>"#,
            hx.0, hx.1, pts(&g.verts[&hx])
        ).unwrap();
    }

    writeln!(s, "</svg>").unwrap();
    s
}

fn generate_rich(g: &Grid) -> String {
    let mut s = String::with_capacity(65536);
    writeln!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">"#).unwrap();

    // Header comments
    write!(s, r#"  <!--
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
      Owner side solid     0% – 42.5%  |  15% blend  |  Neighbor side solid  57.5% – 100%

    Entity counts: {} HCells | {} Quads | {} Tris
    ═══════════════════════════════════════════════════════════════════
  -->
"#, g.hexes.len(), g.quads.len(), g.tris.len()).unwrap();

    // Defs
    writeln!(s, "  <defs>").unwrap();

    // Quad gradients
    for q in &g.quads {
        let gid = format!("gQ-e{}-q{}r{}", q.edge, q.hex.0, q.hex.1);
        writeln!(s, r#"    <linearGradient id="{gid}" gradientUnits="userSpaceOnUse""#).unwrap();
        writeln!(s, r#"        x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}">"#,
            q.grad_from.0, q.grad_from.1, q.grad_to.0, q.grad_to.1).unwrap();
        writeln!(s, r#"      <stop offset="0%"    stop-color="{}"/>"#, q.color_a).unwrap();
        writeln!(s, r#"      <stop offset="42.5%" stop-color="{}"/>"#, q.color_a).unwrap();
        writeln!(s, r#"      <stop offset="57.5%" stop-color="{}"/>"#, q.color_b).unwrap();
        writeln!(s, r#"      <stop offset="100%"  stop-color="{}"/>"#, q.color_b).unwrap();
        writeln!(s, "    </linearGradient>").unwrap();
    }

    // Tri gradients
    for tri in &g.tris {
        for j in 0..3usize {
            if j == tri.base_idx {
                continue;
            }
            let corner = tri.points[j];
            let opp_pts: Vec<Pt> = (0..3).filter(|&k| k != j).map(|k| tri.points[k]).collect();
            let opp = mid(opp_pts[0], opp_pts[1]);
            let col = &tri.colors[j];
            let gid = format!("gT{}-v{}-q{}r{}", j, tri.vertex, tri.hex.0, tri.hex.1);
            writeln!(s, r#"    <linearGradient id="{gid}" gradientUnits="userSpaceOnUse""#).unwrap();
            writeln!(s, r#"        x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}">"#,
                corner.0, corner.1, opp.0, opp.1).unwrap();
            writeln!(s, r#"      <stop offset="0%"    stop-color="{col}" stop-opacity="1"/>"#).unwrap();
            writeln!(s, r#"      <stop offset="42.5%" stop-color="{col}" stop-opacity="1"/>"#).unwrap();
            writeln!(s, r#"      <stop offset="57.5%" stop-color="{col}" stop-opacity="0"/>"#).unwrap();
            writeln!(s, r#"      <stop offset="100%"  stop-color="{col}" stop-opacity="0"/>"#).unwrap();
            writeln!(s, "    </linearGradient>").unwrap();
        }
    }

    writeln!(s, "  </defs>").unwrap();
    writeln!(s, r#"  <rect width="{W}" height="{H}" rx="8" fill="#050810"/>"#).unwrap();
    writeln!(s).unwrap();

    let stroke = r#" stroke="#fff" stroke-opacity="0.5" stroke-width="1.5" stroke-linejoin="round""#;

    // Quads
    writeln!(s, "  <!-- {} Quad gap-fillers (even edges [0,2,4]) -->", g.quads.len()).unwrap();
    for q in &g.quads {
        let gid = format!("gQ-e{}-q{}r{}", q.edge, q.hex.0, q.hex.1);
        let qid = format!("quad-e{}-q{}r{}", q.edge, q.hex.0, q.hex.1);
        writeln!(s, "  <!-- QuadOwner=HCell({},{}) edge{} → HCell({},{}) -->",
            q.hex.0, q.hex.1, q.edge, q.neighbor.0, q.neighbor.1).unwrap();
        writeln!(s, r#"  <polygon id="{qid}" points="{}""#, pts(&q.points)).unwrap();
        writeln!(s, r#"           fill="url(#{gid})"{stroke}/>"#).unwrap();
    }
    writeln!(s).unwrap();

    // Tris
    writeln!(s, "  <!-- {} Tri gap-fillers (triple-junction vertices) -->", g.tris.len()).unwrap();
    for tri in &g.tris {
        let tid = format!("tri-v{}-q{}r{}", tri.vertex, tri.hex.0, tri.hex.1);
        let p = pts(&tri.points);
        let base_color = &tri.colors[tri.base_idx];
        writeln!(s, "  <!-- TriOwner=HCell({},{}) v{} base=corner{} -->",
            tri.hex.0, tri.hex.1, tri.vertex, tri.base_idx).unwrap();
        writeln!(s, r#"  <polygon id="{tid}" points="{p}" fill="{base_color}"/>"#).unwrap();
        for j in 0..3usize {
            if j == tri.base_idx {
                continue;
            }
            let gid = format!("gT{}-v{}-q{}r{}", j, tri.vertex, tri.hex.0, tri.hex.1);
            writeln!(s, r#"  <polygon points="{p}" fill="url(#{gid})" opacity="0.9"/>"#).unwrap();
        }
        writeln!(s, r#"  <polygon points="{p}" fill="none"{stroke}/>"#).unwrap();
    }
    writeln!(s).unwrap();

    // Hex faces
    writeln!(s, "  <!-- {} HCell hex faces -->", g.hexes.len()).unwrap();
    for &hx in &g.hexes {
        let color = &g.colors[&hx];
        let p = pts(&g.verts[&hx]);
        writeln!(s, r#"  <polygon id="hcell-q{}r{}" points="{p}""#, hx.0, hx.1).unwrap();
        writeln!(s, r#"           fill="{color}" fill-opacity="0.85"{stroke}/>"#).unwrap();
    }
    writeln!(s).unwrap();

    // Junction dots
    writeln!(s, "  <g fill=\"#fff\" opacity=\"0.4\">").unwrap();
    let mut seen = HashSet::new();
    for tri in &g.tris {
        for &pt in &tri.points {
            let key = (
                (pt.0 * 10.0).round() as i64,
                (pt.1 * 10.0).round() as i64,
            );
            if seen.insert(key) {
                writeln!(s, r#"    <circle cx="{:.1}" cy="{:.1}" r="1.2"/>"#, pt.0, pt.1).unwrap();
            }
        }
    }
    writeln!(s, "  </g>").unwrap();
    writeln!(s, "</svg>").unwrap();
    s
}

// ── Main ─────────────────────────────────────────────────────────

fn main() {
    let rich = std::env::args().any(|a| a == "--rich");

    let grid = build_grid();

    let lean = generate_lean(&grid);
    fs::write("materials-distro-demo.svg", &lean).expect("write lean SVG");
    eprintln!(
        "  materials-distro-demo.svg       (lean: {} lines)",
        lean.lines().count()
    );

    if rich {
        let rich_svg = generate_rich(&grid);
        fs::write("materials-distro-demo-rich.svg", &rich_svg).expect("write rich SVG");
        eprintln!(
            "  materials-distro-demo-rich.svg  (rich: {} lines)",
            rich_svg.lines().count()
        );
    }

    eprintln!(
        "  {} HCells | {} Quads | {} Tris",
        grid.hexes.len(),
        grid.quads.len(),
        grid.tris.len()
    );
}
