# hex-grid

## 🌐 [**Live WebGL demo →**](https://dynnamitt.github.io/hex-terrain/svg/)

Interactive three.js terrain viewer (orbit / zoom / pan) rendered from the crate's own `--json` output. The same page also shows the plain and rich SVG variants.

---

Hex grid geometry library: noise-driven terrain layout, spatial math, and gap analysis.

Zero game-engine dependencies -- uses `glam` for vectors, `hexx` for hex coordinates, and `noise` for Fbm terrain generation.

## What it does

- **`HGridLayout`** -- builds a hex grid from `HGridSettings`, sampling Fbm noise for per-hex heights and radii. Provides vertex computation, coordinate conversion, and IDW height interpolation.
- **Gap analysis** -- `gap_filler` counts quad/tri gaps using even-edge ownership rules. `quad_corner_indices` maps edge indices to the four corners forming each quad.
- **Pure math helpers** -- `edge_cuboid_transform`, `gap_vertex_data`, `map_noise_to_range`, `idw_interpolate_height`.

## Usage

```rust
use hex_grid::{HGridLayout, HGridSettings};

let settings = HGridSettings { radius: 10, ..HGridSettings::default() };
let layout = HGridLayout::from_settings(&settings);

let height = layout.height(&hexx::Hex::ZERO);
let vertex = layout.vertex(hexx::Hex::ZERO, 0);
let ground = layout.interpolate_height(glam::Vec2::new(5.0, 3.0));
```

## Preview example

The `svg` example renders the grid to stdout in three formats. It's the same binary that drives the [live WebGL demo](https://dynnamitt.github.io/hex-terrain/svg/):

```sh
cargo run -p hex-grid --example svg > grid.svg                 # plain SVG (gray, no labels)
cargo run -p hex-grid --example svg -- 5 2.0 --rich > grid.svg # green fill, outlined strokes, height labels
cargo run -p hex-grid --example svg -- 5 2.0 --json > grid.json # {hexes, edges} for three.js consumption
```

To build the whole preview page (both SVGs + JSON + HTML) locally, use the root Makefile target:

```sh
make svg-preview   # writes into target/svg-preview/
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `glam` 0.30 | Vec2, Vec3, Quat |
| `hexx` 0.24 | Hex coordinates, layouts, shapes |
| `noise` 0.9 | Fbm/Perlin terrain generation |
