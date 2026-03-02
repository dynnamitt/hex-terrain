# CLAUDE.md — hex-terrain

## Project Overview

Bevy 0.18 hex terrain viewer with neon edge lighting. Renders a hexagonal grid with noise-derived terrain heights, progressive edge/face reveal as camera moves, and bloom post-processing.

## Build & Run

Use the Makefile for all standard operations:

```bash
make build                         # cargo build
make test                          # unit tests (cargo test)
make e2etest                       # BRP-based e2e tests (launches app)
make clean                         # cargo clean
cargo run                          # default: intro sequence then free-fly
cargo run -- --debug               # verbose intro logging (DebugFlag resource)
cargo run -- --intro-duration 5    # override tilt-up duration (seconds)
```

## Architecture

Five modules, each split into files: module root (config + plugin), `entities.rs`, `systems.rs`.
Both terrain modules additionally have `startup_systems.rs` and a layout helper.
`--terrain-mode` CLI flag selects V1 (flower-based) or V2 (height-based, default).

```
src/
  main.rs              # CLI (clap), PlayerPos resource, GameState enum, draw_fps, TerrainMode
  math.rs              # Pure math helpers (noise mapping, easing, normals, stem geometry)
  terrain.rs               # TerrainConfig (GridSettings + FlowerSettings), TerrainPlugin (V1)
    terrain/terrain_hex_layout # TerrainHexLayout: encapsulates HexLayout + heights/radii,
                               # on-demand vertex computation, interpolation, inverse transforms
    terrain/entities           # HexGrid, HexSunDisc, Stem, QuadPetal, TriPetal, QuadLines,
                               # FlowerState, HexEntities, HexCtx, NeonMaterials, PetalRes, PetalCtx
    terrain/startup_systems    # generate_grid (Startup schedule)
    terrain/systems            # sync_initial_altitude, update_player_height, track_player_hex,
                               # reveal_nearby_hexes, highlight_nearby_stems, draw_hex_labels
  h_terrain.rs             # HTerrainConfig (HGridSettings), HTerrainPlugin (V2)
    h_terrain/h_grid_layout    # HGridLayout: encapsulates HexLayout + per-hex heights/radii,
                               # vertex computation, height interpolation
    h_terrain/entities         # HGrid, HCell, Corner, Quad, QuadEdge, Tri,
                               # QuadOwner, QuadPos2Emitter, QuadPos3Emitter, QuadTail,
                               # TriOwner, TriPos1Emitter, TriPos2Emitter
    h_terrain/startup_systems  # generate_h_grid (Startup schedule)
    h_terrain/systems          # update_player_height, sync_initial_altitude
  drone.rs             # DroneConfig, DronePlugin
    drone/entities     # Player, CursorRecentered, DroneInput
    drone/systems      # spawn_drone, fly, hide_cursor, recenter_cursor, toggle_inspector
  intro.rs             # IntroConfig, IntroPlugin
    intro/entities     # IntroSequence, IntroPhase
    intro/systems      # run_intro
```

### Config Resources
Each plugin takes a config struct (e.g. `TerrainPlugin(TerrainConfig::default())`).

- `TerrainConfig` — V1: nested `GridSettings` (radius, spacing, noise, hex radii) + `FlowerSettings` (stem params, edge thickness, reveal radius)
- `HTerrainConfig` — V2: `HGridSettings` (radius, spacing, noise seeds/octaves/scales, height/radius ranges) + `clear_color`
- `DroneConfig` — move speed, mouse sensitivity, spawn_altitude (default 12.0), height lerp, bloom intensity
- `IntroConfig` — tilt-up/down durations, highlight delay, tilt-down angle

### SystemParam Bundles
- `DroneInput` — bundles all `fly()` inputs (time, keys, mouse, scroll, config, player)
- `PetalRes` — bundles `reveal_nearby_hexes()` read-only params (grid query, hex entities, neon materials, config)
- `PetalCtx` — plain struct passed to petal spawn helpers (hex entities, neon, grid, config)

### Other Key Resources
- `PlayerPos` — in main.rs: drone writes xz + altitude, terrain writes y
- `GameState` — States enum: `Intro`, `Running`, `Inspecting`
- `HexGrid` — V1 Component, single entity parenting all HexSunDiscs; wraps `TerrainHexLayout`
- `TerrainHexLayout` — V1: encapsulates `HexLayout` + per-hex heights/radii; computes vertices on demand via `vertex(hex, index)`; provides `interpolate_height`, `inverse_transform`, `find_equivalent_vertex`
- `HexEntities` — V1: maps `Hex` → `Entity` for all HexSunDisc entities
- `NeonMaterials` — V1: edge (emissive cyan) + gap face (dark) materials
- `FlowerState` — V1: per-hex reveal state: `Naked` → `Revealed` → `PlayerAbove`; helper methods `needs_petals()`, `demote()`, `promote()`, `fill_petals()`
- `HGrid` — V2 Component, single entity parenting all HCells; wraps `HGridLayout`
- `HGridLayout` — V2: encapsulates `HexLayout` + per-hex heights/radii; `vertex()`, `interpolate_height()`

### Entity Hierarchy

**V1 (terrain)**
```
HexGrid (Component + Transform + Visibility)
  └── HexSunDisc (per hex, scaled by radius)
        ├── Stem (local-space child)
        ├── QuadPetal (even edges 0,2,4 → neighbor)
        │     └── QuadLines (cuboid mesh children)
        └── TriPetal (vertices 0,1 → two neighbors)
```

**V2 (h_terrain)**
```
HGrid (Component + Transform + Visibility)
  └── HCell (per hex, positioned at center + noise height)
        ├── Mesh3d (hex face: orange PlaneMeshBuilder, scaled by radius)
        ├── Corner ×6 (pivot-point children at vertex offsets)
        │     ├── Quad (gap mesh child of QuadOwner corners, even edges)
        │     │     └── QuadEdge ×4 (emissive cyan cuboid edge lines)
        │     └── Tri (gap mesh child of TriOwner corners, vertices 0,1)
```

### System Order
**Startup**: `spawn_drone` → `generate_grid`
**OnEnter(Running)**: `sync_initial_altitude`, `trigger_initial_reveal`
**Update** (via `TerrainSet` pipeline): `PlayerHeight` → `TrackHex` → `RevealPetals` → `Visuals`
- `update_player_height` (Running only)
- `track_player_hex` (Running | Intro, requires HexEntities)
- `reveal_nearby_hexes` (Running only, requires HexGrid)
- `highlight_nearby_stems` (always, skips when xz movement < 0.1)

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
See `TerrainConfig` (with `GridSettings` + `FlowerSettings`), `DroneConfig`, `IntroConfig`.

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

### ECS change detection over `Local` bookkeeping
Prefer Bevy's built-in change detection (`Ref<T>::is_changed()`, `Mut<T>::is_changed()`) over `Local<bool>` / `Local<Option<T>>` for tracking state transitions between frames. When a prior system already mutates a component (e.g. `track_player_hex` promotes `FlowerState`), downstream systems can detect that via `is_changed()` — no manual diffing needed. For state-transition edge cases (system wasn't running when the change happened), use `OnEnter` + `set_changed()` to seed detection.

### Corner marker uniqueness (V2 gap geometry)

The even-edge `[0,2,4]` ownership rule and vertex canonical ownership guarantee that each Corner entity receives each marker component type **at most once**:

| Marker | Corners (per hex) | Mechanism |
|---|---|---|
| `QuadOwner` | 0, 2, 4 | `vertex_dirs[0]` for even edges |
| `QuadTail` | 5, 1, 3 | `vertex_dirs[1]` for even edges |
| `QuadPos2Emitter(Entity)` | 0, 2, 4 | neighbor's `opp_vertex_dirs[1]` only fires for even spawn edges |
| `QuadPos3Emitter(Entity)` | 1, 3, 5 | neighbor's `opp_vertex_dirs[0]` only fires for even spawn edges |
| `TriOwner` | 0, 1 | canonical `coords[0] == hex` for vertex indices 0, 1 |
| `TriPos1Emitter(Entity)` | at most once | same canonical ownership |
| `TriPos2Emitter(Entity)` | at most once | same canonical ownership |

A corner *can* hold multiple *different* marker types simultaneously (e.g. `QuadOwner` + `TriOwner` on corner 0). This is safe because they are distinct component types in ECS. The `PosXEmitter` markers are single-value tuples holding the `Entity` of the gap mesh they contribute to, enabling direct lookup without hierarchy traversal.

## Formatting

No project-specific formatter configured. Standard `cargo fmt`.

## Module Dependency Graph

```
       main (PlayerPos, GameState, TerrainMode)
      / |  \
drone terrain h_terrain
  |     |
intro  math
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
- QuadPetal count used as indirect GameState proof (0 = Intro, 57 = Running)

## MCP Debugger

`RemotePlugin` is enabled. Install `bevy_debugger_mcp` (`cargo install bevy_debugger_mcp`) and configure in `~/.claude/claude_code_config.json` to inspect ECS state at runtime.

## Design Doc

See `PLAN.md` for the full design document including geometry model, vertex height strategies, and verification checklist.
