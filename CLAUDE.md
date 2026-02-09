# CLAUDE.md — hex-terrain

## Project Overview

Bevy 0.18 hex terrain viewer with neon edge lighting. Renders a hexagonal grid with noise-derived terrain heights, progressive edge/face reveal as camera moves, and bloom post-processing. Uses local `hexx` library for hex math.

## Build & Run

```bash
cargo build
cargo run                                    # defaults: --mode full --height-mode smooth
cargo run -- --mode perimeter                # hex outlines only
cargo run -- --mode cross-gap                # gap edges only
cargo run -- --mode full --height-mode blocky # flat hex plateaus
```

## Architecture

```
src/
  main.rs      # CLI (clap), plugin registration, AppConfig resource, RenderMode/HeightMode enums
  visuals.rs   # Camera3d + Hdr + Bloom + Tonemapping, NeonMaterials resource, clear color
  grid.rs      # HexGrid resource, noise heights (Fbm<Perlin>), vertex positions, hex face meshes
  camera.rs    # TerrainCamera, WASD + mouse yaw, vertex-height interpolation, CameraCell tracking
  edges.rs     # Progressive edge/face spawning, DrawnCells, perimeter/cross-gap/full render modes
```

### Per-Plugin Config Resources
Each plugin takes a config struct via its constructor (e.g. `GridPlugin(GridConfig::default())`).
Configs are inserted as ECS resources and read by systems via `Res<XxxConfig>`.

- `GridConfig` — grid radius, spacing, noise seeds/octaves/scales, pole params
- `CameraConfig` — move speed, mouse sensitivity, height offset/lerp, edge margin
- `EdgesConfig` — edge thickness, reveal radius
- `IntroConfig` — tilt-up/down durations, highlight delay, tilt-down angle
- `VisualsConfig` — bloom intensity

### Other Key Resources
- `AppConfig` — render mode (from CLI)
- `HexGrid` — layout, heights map, vertex_positions map
- `ActiveNeonMaterials` — edge (emissive cyan), hex face (dark), gap face (dark) materials
- `CameraCell` — current hex under camera, change detection
- `DrawnCells` — tracks revealed hex cells to avoid duplicate spawning

### System Order
**Startup**: `setup_visuals` -> `generate_grid` -> `draw_initial_cell`
**Update**: `move_camera` -> `track_camera_cell` -> `spawn_cell_geometry`

## Dependencies

- `bevy` 0.18 — selective features, no default (see Cargo.toml for full list)
- `hexx` — local path `../hexx`, features = ["bevy"]. Hex coordinates, layouts, mesh builders.
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
See `GridConfig`, `CameraConfig`, `EdgesConfig`, `IntroConfig`, `VisualsConfig`.

## Formatting

No project-specific formatter configured. Standard `cargo fmt`.

## MCP Debugger

`RemotePlugin` is enabled. Install `bevy_debugger_mcp` (`cargo install bevy_debugger_mcp`) and configure in `~/.claude/claude_code_config.json` to inspect ECS state at runtime.

## Design Doc

See `PLAN.md` for the full design document including geometry model, vertex height strategies, and verification checklist.
