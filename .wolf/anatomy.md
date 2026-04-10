# anatomy.md

> Auto-maintained by OpenWolf. Last scanned: 2026-04-10T18:31:54.783Z
> Files: 67 tracked | Anatomy hits: 0 | Misses: 0

## ../../../../../home/kdm/.claude/plans/

- `hashed-wiggling-rainbow.md` — Plan: Lazy FovMaterial — only FoV-ring faces get the custom shader (~1134 tok)
- `humming-toasting-lagoon.md` — Plan: Extract Shader Magic Numbers to Config/Constants (~1644 tok)

## ./

- `.gitignore` — Git ignore rules (~12 tok)
- `.mcp.json` (~30 tok)
- `Cargo.toml` — Rust package manifest (~439 tok)
- `CLAUDE.md` — OpenWolf (~5517 tok)
- `LICENSE` — Project license (~9374 tok)
- `Makefile` (~484 tok)
- `PLAN.md` — Plan: Hex Terrain Viewer (Bevy 0.18) (~3269 tok)
- `README.md` — Project documentation (~581 tok)
- `rust-analyzer.toml` (~9 tok)
- `UPDATES.md` — Updates (~557 tok)

## .claude/

- `settings.json` (~830 tok)
- `settings.local.json` (~15 tok)

## .claude/agents/

- `bevy-api-checker.md` — Declares signatures (~119 tok)

## .claude/rules/

- `openwolf.md` (~313 tok)

## .claude/skills/bevy-query/

- `SKILL.md` — Methods (Bevy 0.18) (~688 tok)

## .claude/skills/bevy-query/references/

- `recipes.md` — BRP Recipes (~780 tok)
- `response-format.md` — BRP Response Format (~580 tok)
- `type-paths.md` — BRP Type Paths (~883 tok)

## .claude/skills/cargo-check/

- `SKILL.md` (~55 tok)

## .claude/skills/release/

- `SKILL.md` — Steps (~476 tok)

## .github/workflows/

- `pages.yml` — CI: Pages (~989 tok)
- `rust.yml` — CI: Rust (~267 tok)
- `svg-preview.yml` — CI: SVG Preview (~367 tok)

## assets/

- `shimeji.glb` (~2964 tok)

## assets/shaders/

- `fov_overlay.wgsl` — import bevy_pbr::{ (~1616 tok)

## crates/flora/

- `Cargo.toml` — Rust package manifest (~97 tok)
- `README.md` — Project documentation (~340 tok)

## crates/flora/src/

- `cluster.rs` — Shimeji mushroom cluster spawning. (~1173 tok)
- `lib.rs` — Procedural flora meshes and cluster spawning for hex-terrain. (~1027 tok)
- `mesh.rs` — Procedural hex-mushroom mesh generation. (~2496 tok)

## crates/hex-grid/

- `Cargo.toml` — Rust package manifest (~59 tok)
- `README.md` — Project documentation (~367 tok)

## crates/hex-grid/examples/

- `svg.rs` — Renders the hex grid as an SVG file to stdout. (~1351 tok)

## crates/hex-grid/src/

- `layout.rs` — Hex grid layout: spatial mapping, noise-driven heights/radii, vertex computation. (~2913 tok)
- `lib.rs` — Hex grid geometry: layout, noise-driven terrain, and pure math helpers. (~122 tok)
- `math.rs` — Pure computation helpers for hex-grid geometry. (~3560 tok)

## crates/mesh-gradient/

- `Cargo.toml` — Rust package manifest (~205 tok)
- `README.md` — Project documentation (~462 tok)

## crates/mesh-gradient/examples/

- `visual.rs` — Visual test: quad gradient + tri falloff comparison. (~2287 tok)

## crates/mesh-gradient/src/

- `lib.rs` — Procedural gradient materials for gap meshes between hex tiles. (~3331 tok)

## src/

- `drone.rs` — First-person drone controller. (~1492 tok)
- `h_terrain.rs` — Height-based terrain: pivot-point grid with per-hex corners. (~2498 tok)
- `intro.rs` — Intro camera sequence played at startup. (~405 tok)
- `main.rs` — Hex terrain viewer with neon edge lighting. (~1983 tok)
- `math.rs` — Cross-module computation helpers. (~676 tok)

## src/drone/

- `entities.rs` — Marker component for the player-controlled drone entity. (~574 tok)
- `materials.rs` — [derive(Resource)] (~214 tok)
- `systems.rs` — [cfg(not(target_arch = "wasm32"))] (~5307 tok)
- `tests.rs` — ECS integration tests for drone startup and runtime systems. (~4444 tok)

## src/h_terrain/

- `biome.rs` — Biome: configurable mineral distribution for terrain generation. (~952 tok)
- `biomes.rs` — Named biome presets built from the current mineral set. (~738 tok)
- `entities.rs` — Entity types for height-based terrain. (~1524 tok)
- `flora_spawn.rs` — Flora cluster spawning during terrain generation. (~642 tok)
- `fov_overlay.rs` — FoV shader overlay material extension. (~421 tok)
- `gaps.rs` — Quad and Tri gap geometry: spawning, mesh construction, index math. (~7315 tok)
- `h_grid_layout.rs` — Re-exports [`hex_grid::HGridLayout`] and bridges from h_terrain's [`HGridSettings`]. (~101 tok)
- `materials.rs` — Centralized material definitions and FoV material systems for height-based terrain. (~3410 tok)
- `mineral.rs` — Mineral types with per-variant visual properties and scarcity weights. (~1223 tok)
- `startup_systems.rs` — Startup systems for height-based terrain. (~2297 tok)
- `systems.rs` — Runtime systems for height-based terrain. (~2363 tok)
- `tests.rs` — ECS integration tests for h_terrain startup and runtime systems. (~3741 tok)

## web/

- `history.html` — Hex Terrain — versions (~256 tok)
- `index.html` — Hex Terrain (~1098 tok)
- `root-index.html` — Hex Terrain (~69 tok)
- `svg-preview.html` — hex-grid preview (~232 tok)
