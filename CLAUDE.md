# CLAUDE.md — hex-terrain

## Project Overview

Bevy 0.18 hex terrain viewer with neon edge lighting and laser aiming. Renders a hexagonal grid with noise-derived terrain heights, progressive edge/face reveal as camera moves, crosshair aiming with laser fire, and bloom post-processing. Supports both native (Linux/X11) and WASM (WebGL2) targets.

## Build & Run

Use the Makefile for all standard operations:

```bash
make build                         # cargo build
make test                          # unit tests (cargo test)
make coverage                      # tarpaulin HTML coverage report
make wasm                          # WASM build + wasm-bindgen + copy web/index.html
make serve                         # wasm + python3 HTTP server on :8080
make clean                         # cargo clean
cargo run                          # default: intro → arming → free-fly
cargo run -- --debug               # verbose intro logging (DebugFlag resource) + FPS overlay
cargo run -- --intro-duration 5    # override tilt-up duration (seconds)
```

## Architecture

Four modules, each split into files: module root (config + plugin), `entities.rs`, `systems.rs`.
The terrain module additionally has `startup_systems.rs`, a layout helper, terrain-specific math, and a materials module. The drone module has its own materials module.

```
src/
  main.rs              # CLI (clap), PlayerPos, PlayerMoved, GroundLevel, GameState,
                       # TerrainSeededPhase, DebugFlag, draw_fps, toggle_inspector
  math.rs              # Cross-module helpers (ease_out_cubic, clamp_pitch)
  h_terrain.rs             # HTerrainConfig (HGridSettings), HTerrainPlugin, HTerrainPhase
    h_terrain/h_grid_layout    # HGridLayout: encapsulates HexLayout + per-hex heights/radii,
                               # vertex computation, height interpolation
    h_terrain/math             # Terrain-specific math: map_noise_to_range, compute_normal,
                               # gap_filler, idw_interpolate_height, edge_cuboid_transform,
                               # quad_corner_indices, build_gap_mesh
    h_terrain/materials        # OrigPalette, FovPalette, TerrainMaterials resource,
                               # FovChanges/SightParams SystemParam bundles,
                               # start_fov_transitions, animate_fov_transitions, track_in_sight
    h_terrain/entities         # HGrid, HCell, HexFace, Corner, Quad, QuadEdge, Tri,
                               # QuadOwner, QuadPos2Emitter, QuadPos3Emitter, QuadTail,
                               # TriOwner, TriPos1Emitter, TriPos2Emitter,
                               # InFov, FovTransition, InSight, PreSightMaterial
    h_terrain/startup_systems  # generate_h_grid, seed_ground_level, verify_gap_counts
    h_terrain/systems          # update_ground_level, track_player_fov
    h_terrain/tests            # ECS integration tests (cfg(test))
  drone.rs             # DroneConfig, DronePlugin
    drone/entities     # Player, Elbow, LaserPipe, LaserRay, ArmingTimer,
                       # CursorRecentered, DroneInput
    drone/materials    # DroneMaterials resource (pipe, laser_ray)
    drone/systems      # create_drone_materials, spawn_drone, fly, arm_pipe,
                       # draw_crosshair, fire_laser,
                       # hide_cursor, recenter_cursor (native),
                       # lock_cursor_on_click (wasm)
    drone/tests        # drone unit tests (cfg(test))
  intro.rs             # IntroConfig, IntroPlugin
    intro/entities     # IntroTimer, IntroPhase
    intro/systems      # run_intro
```

### Config Resources
Each plugin takes a named-struct config (e.g. `HTerrainPlugin { config: ..., ... }`).

- `HTerrainConfig` — `HGridSettings` (radius, fov_reach, spacing, noise seeds/octaves/scales, height/radius ranges) + `clear_color` + `fov_transition_secs`
- `DroneConfig` — move speed, mouse sensitivity, lowest_offset, height lerp, bloom intensity, pipe geometry (offset/length/radius), laser_thickness, arm_duration
- `IntroConfig` — tilt-up/down durations, highlight delay, tilt-down angle

### SystemParam Bundles
- `DroneInput` — bundles all `fly()` inputs (time, keys, mouse, scroll, recentered, config, ground, player, moved)
- `FovChanges` — bundles InFov change-detection queries and cell→HexFace/gap navigation
- `SightParams` — bundles camera raycast, hex face queries, and InSight state for `track_in_sight`

### Other Key Resources
- `PlayerPos` — in main.rs: drone writes xz + offset (above ground)
- `PlayerMoved` — set by drone/intro when position changes; consumed by terrain height systems
- `GroundLevel` — `Option<f32>`: `None` until terrain seeded, then `Some(terrain_height)` under the player
- `GameState` — States enum: `Intro`, `Arming`, `Running`, `Inspecting`
- `DebugFlag` — CLI `--debug` flag; enables FPS overlay and `verify_gap_counts`
- `TerrainMaterials` — material handles for hex faces, gaps, edges, aim highlight (7 handles)
- `DroneMaterials` — material handles for laser pipe and ray
- `HGrid` — Component, single entity parenting all HCells; wraps `HGridLayout`
- `HGridLayout` — encapsulates `HexLayout` + per-hex heights/radii; `vertex()`, `interpolate_height()`

### Color Palettes
- `OrigPalette` — base terrain colors: Hex (olive), Gap (near-black), Edge (azure), Debug (hot pink), ClearColor (navy)
- `FovPalette` — FoV highlight colors: Hex/Edge (bright green), Gap (muted lime), Aim (purple)
- Both implement `From<T> for Color` (base_color) and `From<T> for LinearRgba` (emissive)

### Entity Hierarchy
```
HGrid (Component + Transform + Visibility)
  └── HCell (per hex, positioned at center + noise height)
        ├── HexFace (hex face mesh: PlaneMeshBuilder, scaled by radius)
        ├── Corner ×6 (pivot-point children at vertex offsets)
        │     ├── Quad (gap mesh child of QuadOwner corners, even edges)
        │     │     └── QuadEdge ×4 (emissive cyan cuboid edge lines)
        │     └── Tri (gap mesh child of TriOwner corners, vertices 0,1)

Player (Camera3d + Hdr + Bloom)
  └── Elbow (pivot for pipe swing-in animation)
        └── LaserPipe (cylinder mesh)

LaserRay (root entity, world-space positioned cuboid, Visibility::Hidden until firing)
```

### System Order
**Startup**: `create_drone_materials` → `generate_h_grid` → `seed_ground_level` (in `TerrainSeededPhase`) → `spawn_drone` (after both)
**Startup** (debug only): `verify_gap_counts` (after `generate_h_grid`)
**Update** (via `HTerrainPhase` pipeline, Running only): `UpdateGround` → `TrackFov` → `Highlight` → `Sight`
- `update_ground_level` — sets `GroundLevel` from terrain interpolation (guarded by `PlayerMoved`)
- `track_player_fov` — adds/removes `InFov` on nearby HCells
- `start_fov_transitions` / `animate_fov_transitions` — material color lerp for FoV reveal
- `track_in_sight` — raycasts screen center, tags aimed HexFace with `InSight` + purple material
**Update** (Arming only): `arm_pipe` — animates Elbow rotation, transitions to Running
**Update** (Running only): `draw_crosshair`, `fire_laser` (after Sight phase), `fly` (after `recenter_cursor`)

## Dependencies

- `bevy` 0.18 — selective features, no default (see Cargo.toml for full list)
- `hexx` 0.24 — hex coordinates, layouts, mesh builders (with `bevy` feature for Reflect/Component derives)
- `noise` 0.9 — Fbm<Perlin> terrain generation
- `clap` 4 — CLI argument parsing (optional, native only via `dep:clap`)
- `bevy-inspector-egui` 0.36 + `bevy_egui` 0.39 — dev inspection UI

### Feature Flags
- `native` (default) — `clap`, `bevy/x11`, `bevy/multi_threaded`, `bevy/bevy_remote`
- `web` — `bevy/webgl2` for WASM builds

## Bevy 0.18 Specifics

These differ from earlier Bevy tutorials/docs:
- HDR: use `Hdr` component, not `Camera { hdr: true }`
- Bloom: `bevy::post_process::bloom`, not `bevy::core_pipeline::bloom`
- Events: `MessageReader<T>`, not `EventReader<T>`
- Imports: `bevy::platform::collections::{HashMap, HashSet}`
- Relationships: `ChildOf` component for parent lookups (via `bevy::ecs::relationship::Relationship`)
- Single queries: `Single<&T, With<Marker>>` system param (not `query.single()`)

## Key Default Values

All constants are fields on per-plugin config structs with `Default` impls.
See `HTerrainConfig` (with `HGridSettings`), `DroneConfig`, `IntroConfig`.

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
Group related `Res<T>` params into a struct (e.g. `DroneInput`, `FovChanges`, `SightParams`) to reduce system signature clutter.

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

## WASM Support

The project compiles to WebAssembly with `make wasm`. Platform differences:
- CLI parsing (`clap`) is `#[cfg(not(target_arch = "wasm32"))]`; WASM defaults to `debug=false`
- `RemotePlugin` / `RemoteHttpPlugin` are native-only
- Cursor handling: native uses `hide_cursor` + `recenter_cursor`; WASM uses `lock_cursor_on_click` (browser requires user gesture for pointer lock)
- Canvas binding: `#game-canvas` selector, `fit_canvas_to_parent: true`
- WASM build profile: `wasm-release` (inherits release, `opt-level = "s"`, thin LTO, strip debuginfo)

## Release Notes

`UPDATES.md` is the source of truth for release notes. The loading overlay in `web/index.html` displays the same bullets — keep both in sync when adding entries.

## Formatting

No project-specific formatter configured. Standard `cargo fmt`.

## Module Dependency Graph

```
       main (PlayerPos, PlayerMoved, GroundLevel, GameState, TerrainSeededPhase)
      / |  \
drone  h_terrain  math
  |
intro
```

## Testing

### Unit / Integration Tests

`h_terrain/tests.rs` contains ECS integration tests that run h_terrain systems in a headless Bevy `App` (no window/renderer). The `test_app()` helper wires up `MinimalPlugins` + `AssetPlugin`, registers all h_terrain startup and update systems, and forces `GameState::Running`. Tests cover:
- Startup entity counts (HGrid, HCell, Corner, Quad, Tri, QuadEdge)
- Gap entity counts matching `math::gap_filler` predictions
- `seed_ground_level` correctness
- `update_ground_level` on player movement
- `track_player_fov` at origin, after hex boundary crossing, and at grid edge
- `start_fov_transitions` / `animate_fov_transitions` direction and completion

Additional test modules:
- `h_terrain/math` — unit tests for `gap_filler`, `map_noise_to_range`, `compute_normal`, `idw_interpolate_height`, `edge_cuboid_transform`, `quad_corner_indices`
- `math` — unit tests for `ease_out_cubic`, `clamp_pitch`
- `drone/tests` — drone controller tests

### Coverage

CI generates coverage via **cargo-tarpaulin** and uploads to **Codecov**. Run locally with `make coverage` (produces `tarpaulin-report.html`).

## MCP Debugger

`RemotePlugin` is enabled (native only). Install `bevy_debugger_mcp` (`cargo install bevy_debugger_mcp`) and configure in `~/.claude/claude_code_config.json` to inspect ECS state at runtime.

BRP serialization notes (Bevy 0.18):
- Transform `translation`: `[x, y, z]` array (not `{x, y, z}` object)
- GlobalTransform: 12-float Affine3A array; translation at indices `[9, 10, 11]`
- Name component path: `bevy_ecs::name::Name`
- hexx `Hex` lacks `ReflectSerialize` — use Name-based lookup
- Material handles (`MeshMaterial3d<StandardMaterial>`) can't be read via BRP

## Design Doc

See `PLAN.md` for the full design document including geometry model, vertex height strategies, and verification checklist.
