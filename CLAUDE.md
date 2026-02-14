# CLAUDE.md — hex-terrain

## Project Overview

Bevy 0.18 hex terrain viewer with neon edge lighting. Renders a hexagonal grid with noise-derived terrain heights, progressive edge/face reveal as camera moves, and bloom post-processing.

## Build & Run

```bash
cargo build
cargo run                # default: intro sequence then free-fly
cargo run -- --debug     # start in GameState::Debugging (inspector via Tab)
```

## Architecture

Four modules, each split into three files: module root (config + plugin), `entities.rs`, `systems.rs`.

```
src/
  main.rs              # CLI (clap), PlayerPos resource, GameState enum
  math.rs              # Pure math helpers (noise mapping, easing, normals, pole geometry)
  terrain.rs           # TerrainConfig (GridSettings + PetalSettings), TerrainPlugin
    terrain/entities   # HexGrid, HexSunDisc, HeightPole, QuadLeaf, TriLeaf, PetalEdge,
                       # HexEntities, DrawnCells, ActiveHex, NeonMaterials, PetalRes, LeafCtx
    terrain/systems    # generate_grid, update_player_height, track_active_hex,
                       # spawn_petals, highlight_nearby_poles, draw_hex_labels (debug)
  drone.rs             # DroneConfig, DronePlugin
    drone/entities     # Player, CursorRecentered, DroneInput
    drone/systems      # spawn_drone, fly, hide_cursor, recenter_cursor, toggle_inspector
  intro.rs             # IntroConfig, IntroPlugin
    intro/entities     # IntroSequence, IntroPhase
    intro/systems      # run_intro
```

### Config Resources
Each plugin takes a config struct (e.g. `TerrainPlugin(TerrainConfig::default())`).

- `TerrainConfig` — nested `GridSettings` (radius, spacing, noise, pole params) + `PetalSettings` (edge thickness, reveal radius)
- `DroneConfig` — move speed, mouse sensitivity, spawn_altitude (default 12.0), height lerp, bloom intensity
- `IntroConfig` — tilt-up/down durations, highlight delay, tilt-down angle

### SystemParam Bundles
- `DroneInput` — bundles all `fly()` inputs (time, keys, mouse, scroll, config, player)
- `PetalRes` — bundles `spawn_petals()` read-only params (grid query, hex entities, neon materials, config, active hex)
- `LeafCtx` — plain struct passed to leaf spawn helpers (hex entities, neon, grid, config)

### Other Key Resources
- `PlayerPos` — in main.rs: drone writes xz + altitude, terrain writes y
- `GameState` — States enum: `Intro`, `Running`, `Debugging`
- `HexGrid` — Component (not Resource), single entity parenting all HexSunDiscs
- `HexEntities` — maps `Hex` → `Entity` for all HexSunDisc entities
- `NeonMaterials` — edge (emissive cyan) + gap face (dark) materials
- `ActiveHex` — current hex under player, with change detection
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
**Startup**: `spawn_drone` → `generate_grid`
**Update**: `fly` → `update_player_height` (Running only) → `track_active_hex` (Running | Intro) → `spawn_petals` (Running only) → `highlight_nearby_poles` (always)

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

All constants are fields on per-plugin config structs with `Default` impls.
See `TerrainConfig` (with `GridSettings` + `PetalSettings`), `DroneConfig`, `IntroConfig`.

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

## Module Dependency Graph

```
    main (PlayerPos, GameState)
    / \
drone   terrain
  |       |
intro   math
```

## E2E Testing (BRP)

Tests in `tests/e2e_entity_count.sh` query the running app via Bevy Remote Protocol (`http://127.0.0.1:15702`).

BRP serialization notes (Bevy 0.18):
- Transform `translation`: `[x, y, z]` array (not `{x, y, z}` object)
- GlobalTransform: 12-float Affine3A array; translation at indices `[9, 10, 11]`
- Name component path: `bevy_ecs::name::Name`
- HexSunDisc data doesn't serialize (hexx `Hex` lacks `ReflectSerialize`) — use Name-based lookup
- Material handles (`MeshMaterial3d<StandardMaterial>`) can't be read via BRP
- `GameState` and custom resources not exposed via `world.list_resources`
- QuadLeaf count used as indirect GameState proof (0 = Intro, 57 = Running)

## MCP Debugger

`RemotePlugin` is enabled. Install `bevy_debugger_mcp` (`cargo install bevy_debugger_mcp`) and configure in `~/.claude/claude_code_config.json` to inspect ECS state at runtime.

## Design Doc

See `PLAN.md` for the full design document including geometry model, vertex height strategies, and verification checklist.
