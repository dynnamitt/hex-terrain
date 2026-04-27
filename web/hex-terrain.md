# Vertex deduplication in `hex-terrain.js`

`weldedMesh` (in `hex-terrain.js`) hashes every `[x, y, z]` and
reuses the index for any exact match, building an indexed
`THREE.BufferGeometry` instead of a triangle soup. This document explains
what that buys at the project's current scale and when it would start to
matter for performance.

## Honest summary

At this scale, **dedup is not a render-speed optimization** — it's a
topology/shading correctness move. The word "weld" is a tessellation term,
not a performance term.

## The math at this scale

- Welded mesh: ~150–200 unique vertices, ~300 tris.
- Without dedup: ~700+ unique vertex slots.
- Modern GPUs render millions of tris per frame. 700 → 200 vertices saves
  ~6 KB of position data and a handful of microseconds. Not visible on a
  frame timer.

## Where indexed welded meshes actually pay off

- **100k+ tris with a non-trivial vertex shader** (skinning, vertex
  displacement, morph targets): the GPU's post-transform vertex cache
  reuses shaded vertex outputs when the same vertex is referenced by
  multiple triangles. Welded + indexed lets that cache hit, maybe
  1.2–1.5× win.
- **Memory-constrained targets** (mobile, WebGL on low-end): smaller VBO
  uploads matter more. Still small at ~300 tris.
- **CPU-side raycasts / picking**: fewer verts to test. Three.js'
  `Raycaster` does benefit at large scales.

## What dedup *does* buy in this code

1. **Single connected manifold.** Without bit-exact vertex matching at
   seams, you'd get hairline cracks where a hex face meets a gap quad —
   not from rendering artifacts but from floating-point drift if positions
   were re-emitted from different code paths. The Rust crate's
   `HGridLayout::vertex(hex, i)` (`crates/apicult-desigual/src/layout.rs:143-150`)
   returning identical `Vec3` for shared corners is what makes the weld
   watertight.
2. **Free smooth shading at seams** — when `computeVertexNormals()` runs
   on the indexed mesh, it averages normals across faces sharing a vertex.
   The hex-face-to-gap-quad transition gets a smoothed normal "for free."
   You explicitly opt out via `flatShading: true` on the material;
   three.js then uses per-fragment `dFdx/dFdy` derivatives to fake flat
   normals, which works on welded meshes too. The "flat shading" toggle
   in `hex-terrain.html` exposes both modes.
3. **Cleaner traversal** — `BufferGeometry.attributes.position.count`
   reflects unique points, useful for diagnostics (the live counter in
   `hex-terrain.html` shows the welded count).

## What you'd actually want for speed at production scale

- One `BufferGeometry` per mesh (already the case — single draw call).
- `THREE.BufferGeometryUtils.mergeVertices(geom, tolerance)` to weld
  post-hoc if you ever load unwelded data (e.g. from a glTF).
- `THREE.InstancedMesh` if you ever spawn many copies of the same hex chunk.
- LOD via fewer triangles, not via deduping the same triangles.

## Bottom line

Dedup at our scale is **correctness + ergonomics**, not FPS. If terrain
grows to grid radius 20+ (~thousands of hexes, ~50k+ tris) the indexed
post-transform-cache benefit becomes measurable; below that, rendering
the unwelded version side-by-side won't show a fps difference.
