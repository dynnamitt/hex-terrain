# apicult-desigual

## 🌐 [**Live WebGL demo →**](https://dynnamitt.github.io/hex-terrain/svg/)

Interactive three.js terrain viewer (orbit / zoom / pan) rendered from the crate's own JSON output. The same page also shows the plain and rich SVG variants.

---

Hex grid geometry library: noise-driven terrain layout, spatial math, gap analysis, and SVG/JSON serialization.

Zero game-engine dependencies -- uses `glam` for vectors, `hexx` for hex coordinates, and `noise` for Fbm terrain generation.

## What it does

- **`HGridLayout`** -- builds a hex grid from `HGridSettings`, sampling Fbm noise for per-hex heights and radii. Provides vertex computation, coordinate conversion, IDW height interpolation, and triangulated geometry: `gap_quads`, `gap_tris`, `hex_face_tris`, and the unified `all_tris` stream (gap quads split along the canonical `[v0, v2]` diagonal).
- **Gap analysis** -- `gap_filler` counts quad/tri gaps using even-edge ownership rules. `quad_corner_indices` maps edge indices to the four corners forming each quad.
- **Serialization** -- `SerializeGeo` trait with four implementors: `SvgPlain`, `SvgRich`, `JsonV1` (gen1: `hexes` / `edges` / `quads` / `tris`), `JsonV2` (gen2: `{ version: 2, tris }` -- unified stream from `all_tris`).
- **Pure math helpers** -- `edge_cuboid_transform`, `gap_vertex_data`, `map_noise_to_range`, `idw_interpolate_height`.

## Usage

```rust
use apicult_desigual::{HGridLayout, HGridSettings};

let settings = HGridSettings { radius: 10, ..HGridSettings::default() };
let layout = HGridLayout::from_settings(&settings);

let height = layout.height(&hexx::Hex::ZERO);
let vertex = layout.vertex(hexx::Hex::ZERO, 0);
let ground = layout.interpolate_height(glam::Vec2::new(5.0, 3.0));

// Unified triangle stream — diagonal choice owned here, not on the client.
let tris = layout.all_tris();
```

## Preview example

The `geo_export` example streams the grid to stdout in any of the supported formats. It's the same binary that drives the [live WebGL demo](https://dynnamitt.github.io/hex-terrain/svg/):

```sh
cargo run -p apicult-desigual --example geo_export                                   # plain SVG (gray, no labels)
cargo run -p apicult-desigual --example geo_export -- 5 2.0 --format svg-rich        # green fill, outlined strokes, height labels
cargo run -p apicult-desigual --example geo_export -- 5 2.0 --format json-v1         # gen1: {hexes, edges, quads, tris}
cargo run -p apicult-desigual --example geo_export -- 5 2.0 --format json-v2         # gen2: {version: 2, tris}  — welded-mesh consumers
```

`--rich` and `--json` remain as backward-compat aliases for `--format svg-rich` and `--format json-v1`.

To build the whole preview page (both SVGs + both JSON variants + HTML) locally, use the root Makefile target:

```sh
make svg-preview   # writes into target/svg-preview/
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `glam` 0.30 | Vec2, Vec3, Quat |
| `hexx` 0.24 | Hex coordinates, layouts, shapes |
| `noise` 0.9 | Fbm/Perlin terrain generation |
