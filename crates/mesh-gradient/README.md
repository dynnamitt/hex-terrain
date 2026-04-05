# mesh-gradient

Procedural gradient materials for gap meshes between hex tiles.

Generates Bevy `StandardMaterial`s with `Rgba16Float` gradient textures that blend between two (quad) or three (tri) input materials. No custom shaders -- works with standard PBR rendering and the existing Bevy pipeline.

## What it does

- **`blend_quad`** -- blends two materials along the U axis with a configurable transition band. Same-color inputs skip the texture path entirely.
- **`blend_tri`** -- blends three materials using projected corner weights with selectable falloff curves (`Hotspot` or `Plateau`).
- **`highlight`** -- produces a FoV highlight variant of any gradient material by overbrightening and adding emissive glow. Reuses the same texture handle.

## Why Rgba16Float

Storing linear color values in `Rgba16Float` avoids the u8 quantization and sRGB round-trip that causes visible brightening on dark materials. The GPU samples exact linear values with no encoding artifacts.

## Usage

```rust
use mesh_gradient::{BlendCfg, TriFalloff, blend_quad, blend_tri, highlight};

let cfg = BlendCfg::default(); // band=0.15, quad=32x16, tri=64x64

// Quad: blend two StandardMaterials
let grad = blend_quad(&mat_a, &mat_b, &cfg, &mut images);

// Tri: blend three materials with Plateau falloff
let tri_grad = blend_tri([&m0, &m1, &m2], 0, TriFalloff::Plateau, &cfg, &mut images);

// FoV highlight variant (reuses texture, adds emissive)
let hi = highlight(&grad, 0.15, LinearRgba::new(0.0, 0.1, 0.0, 1.0));
```

## Visual example

Requires the `visual` feature (pulls in Bevy windowing):

```sh
cargo run -p mesh-gradient --example visual --features visual
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `bevy` 0.18 | PBR materials, images, assets (minimal feature set) |
| `half` 2 | f16 encoding for Rgba16Float textures |
