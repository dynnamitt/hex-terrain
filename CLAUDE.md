# CLAUDE.md — hex-terrain

## Project Overview

Bevy 0.18 hex terrain viewer with neon edge lighting. Renders a hexagonal grid with noise-derived terrain heights, progressive edge/face reveal as camera moves, and bloom post-processing.

## Build & Run

```bash
cargo build
cargo run                                    # defaults: --mode full --height-mode smooth
cargo run -- --mode perimeter                # hex outlines only
cargo run -- --mode cross-gap                # gap edges only
cargo run -- --mode full --height-mode blocky # flat hex plateaus
```

## Architecture

Each plugin is split into three files: module root (config + plugin), `entities.rs`, `systems.rs`.

```
src/
  main.rs            # CLI (clap), plugin registration, AppConfig, RenderMode
  math.rs            # Pure math helpers (noise mapping, easing, pole geometry)
  visuals.rs         # VisualsConfig, VisualsPlugin
    visuals/entities # ActiveNeonMaterials
    visuals/systems  # setup_visuals (Camera3d + Hdr + Bloom + materials)
  grid.rs            # GridConfig, GridPlugin
    grid/entities    # HexGrid (Component, parents all HexSunDiscs)
    grid/systems     # generate_grid, fade_nearby_poles, draw_hex_labels (debug)
  camera.rs          # CameraConfig, CameraPlugin
    camera/entities  # TerrainCamera, CameraCell, CursorRecentered
    camera/systems   # move_camera, track_camera_cell, interpolate_height, cursor mgmt
  intro.rs           # IntroConfig, IntroPlugin
    intro/entities   # IntroSequence, IntroPhase
    intro/systems    # run_intro
  petals.rs          # PetalsConfig, PetalsPlugin
    petals/entities  # HeightPole, HexSunDisc, QuadLeaf, TriLeaf, PetalEdge, HexEntities, DrawnCells
    petals/systems   # spawn_petals + leaf/mesh/pure helpers
```

### Per-Plugin Config Resources
Each plugin takes a config struct via its constructor (e.g. `GridPlugin(GridConfig::default())`).
Configs are inserted as ECS resources and read by systems via `Res<XxxConfig>`.

- `GridConfig` — grid radius, spacing, noise seeds/octaves/scales, pole params
- `CameraConfig` — move speed, mouse sensitivity, height offset/lerp, edge margin
- `PetalsConfig` — edge thickness, reveal radius
- `IntroConfig` — tilt-up/down durations, highlight delay, tilt-down angle
- `VisualsConfig` — bloom intensity

### Other Key Resources
- `AppConfig` — render mode (from CLI)
- `HexGrid` — Component (not Resource), single entity parenting all HexSunDiscs
- `HexEntities` — maps `Hex` → `Entity` for all HexSunDisc entities
- `ActiveNeonMaterials` — edge (emissive cyan), hex face (dark), gap face (dark) materials
- `CameraCell` — current hex under camera, change detection
- `DrawnCells` — tracks revealed hex cells to avoid duplicate petal spawning

### Entity Hierarchy
```
HexGrid (Component + Transform + Visibility)
  └── HexSunDisc (per hex, scaled by radius)
        ├── HeightPole (local-space child)
        ├── QuadLeaf (even edges 0,2,4 → neighbor)
        │     └── PetalEdge (cuboid mesh children)
        └── TriLeaf (vertices 0,1 → two neighbors)
```

### System Order
**Startup**: `setup_visuals` → `generate_grid`
**Update**: `move_camera` → `track_camera_cell` → `spawn_petals`

## Dependencies

- `bevy` 0.18 — selective features, no default (see Cargo.toml for full list)
- `hexx` 0.24 — hex coordinates, layouts, mesh builders (with `bevy` feature for Reflect/Component derives)
- `noise` 0.9 — Fbm<Perlin> terrain generation
- `clap` 4 — CLI argument parsing
- `bevy-inspector-egui` 0.36 + `bevy_egui` 0.39 — dev inspection UI

## Bevy 0.18 Specifics

These differ from earlier Bevy tutorials/docs:
- HDR: use `Hdr` component, not `Camera { hdr: true }`
- Bloom: `bevy::post_process::bloom`, not `bevy::core_pipeline::bloom`
- Events: `MessageReader<T>`, not `EventReader<T>`
- Imports: `bevy::platform::collections::{HashMap, HashSet}`

## Key Default Values

All constants are now fields on per-plugin config structs with `Default` impls.
See `GridConfig`, `CameraConfig`, `PetalsConfig`, `IntroConfig`, `VisualsConfig`.

## Code Patterns

### Guard-heavy helpers → `-> Option<()>` + `?`
When a function has multiple early-return guards before side effects, use `-> Option<()>` with `?`:
- `contains_key` → `.get(&key)?` (discard value)
- `let Some(&x) = map.get(...) else { return }` → `let &x = map.get(...)?`
- Boolean guards → `condition.then_some(())?`
- Mode/enum guard at top: use explicit `return None` for clarity

### Nested `if let` → chained `.and_then()`
Flatten `if let { if let { if let {` pyramids into a single `if let` with `.and_then()`:
```rust
if let Some(name) = opt_res.as_ref().and_then(|r| r.map.get(&key)).and_then(|&e| query.get(e).ok()) {
    println!("{name}");
}
```

### `#[derive(SystemParam)]` for resource bundles
Group related `Res<T>` params into a struct (e.g. `PetalRes`) to reduce system signature clutter.

## Formatting

No project-specific formatter configured. Standard `cargo fmt`.

## MCP Debugger

`RemotePlugin` is enabled. Install `bevy_debugger_mcp` (`cargo install bevy_debugger_mcp`) and configure in `~/.claude/claude_code_config.json` to inspect ECS state at runtime.

## Design Doc

See `PLAN.md` for the full design document including geometry model, vertex height strategies, and verification checklist.
