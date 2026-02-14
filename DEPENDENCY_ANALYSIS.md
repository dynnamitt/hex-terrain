# Module Dependency Analysis

## Current Dependency Graph

```
            main
           / | \
     visuals intro  AppConfig/RenderMode
       |  \    |  \        |
       |   \   |  camera   |
       |    \  |  / |      |
       grid  \ | /  |      |
         \    \|/   |      |
          petals <--+------+
```

Arrows = "depends on". `petals` is the sink — it touches almost every other module.

## Problems

### 1. `visuals` is a god-module for materials

`ActiveNeonMaterials` is created in `visuals::setup_visuals` and consumed by both
`grid::generate_grid` and `petals::spawn_petals`. This forces a hard startup ordering
constraint and makes grid/petals depend on visuals for their own materials.

Also spawns the `Camera3d` entity with `Player` — a drone concept. Conceptual
cycle: visuals depends on camera for the marker, camera depends on visuals for the entity.

### 2. `petals` depends on everything

5+ cross-module imports. The `PetalRes` SystemParam bundles them but doesn't reduce
coupling.

### 3. `grid ↔ petals` cycle

`HexEntities`, `HexSunDisc`, `HeightPole` defined in petals, spawned/created by grid.

### 4. `interpolate_height` misplaced

Pure fn on `HexGrid`, lives in camera but also called by intro.

---

## Agreed Refactoring Plan

### Merge grid + petals + visuals → `terrain`

The `terrain` module absorbs all terrain-related concerns:
- Grid generation (hex layout, noise heights, vertex positions)
- Petal spawning (edges, faces, leaves)
- Materials (each created inline, no shared material resource)
- Clear color setup
- All shared types (HexGrid, HexEntities, HexSunDisc, HeightPole, etc.)

Structure:
```
src/
  terrain.rs              # TerrainConfig (merged Grid+Petals fields) + TerrainPlugin
  terrain/entities.rs     # HexGrid, HexEntities, HexSunDisc, HeightPole,
                          #   QuadLeaf, TriLeaf, PetalEdge, DrawnCells
  terrain/systems.rs      # generate_grid, spawn_petals, fade_nearby_poles,
                          #   interpolate_height, material creation
```

### Drone owns its entity

Drone plugin spawns Camera3d + Player + Hdr + Bloom in its own startup system.
Bloom intensity becomes a field on `DroneConfig`.

### Remove visuals module entirely

Responsibilities spread:
- Camera3d + Hdr + Bloom → drone plugin
- Materials → terrain plugin (created per-system, no shared resource)
- ClearColor → terrain plugin

### Simplify CLI to `--debug` only

Remove `--mode` and `--height-mode` CLI args. `RenderMode` hardcoded to `Full`.
CLI only parses a `--debug` flag.

`AppConfig`, `RenderMode` enum, `InspectorActive` removed entirely.

### GameState enum replaces IntroSequence + InspectorActive

```rust
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    // WelcomeMenu,   // future
    #[default]
    Intro,
    Running,
    Debugging,       // only reachable when --debug flag is set
    // Paused,        // future
}
```

Defined in `main.rs`. Replaces:
- `IntroSequence` resource (state-tracking part — `done: bool`, phase enum)
- `InspectorActive` resource

Intro animation timing data (tilt durations, elapsed time) stays in a local
resource within the intro module (e.g. `IntroTimer`), but phase/done tracking
is gone — intro system transitions `GameState::Intro → GameState::Running`
when animation completes.

`run_if` conditions become idiomatic Bevy state checks:
- `run_if(|intro| intro.done)` → `in_state(GameState::Running)`
- `run_if(|active| !active.0)` → `not(in_state(GameState::Debugging))`
- `run_if(|active| active.0)` → `in_state(GameState::Debugging)`
- Tab keybind toggles `Running ↔ Debugging` (only when `--debug`)

`terrain::spawn_petals` currently waits for `IntroSequence.initial_draw_triggered`
→ instead runs in `OnEnter(GameState::Running)` or gated by `in_state(Running)`.

### Single TerrainConfig

Single config with nested sub-structs for logical grouping:
```rust
pub struct TerrainConfig {
    pub grid: GridSettings,
    pub petals: PetalSettings,
    pub clear_color: Color,
}

pub struct GridSettings {
    pub radius: u32,
    pub hex_spacing: f32,
    pub noise_seed: u32,
    // ... etc
}

pub struct PetalSettings {
    pub edge_thickness: f32,
    pub reveal_radius: u32,
    // ... etc
}

```

### Decouple drone from terrain via shared resources in main

Drone and intro should have zero terrain/hex awareness. Data flows through
a single shared resource defined in `main.rs`:

```rust
/// Drone/intro write xz + altitude. Terrain writes y (height + altitude).
#[derive(Resource)]
pub struct PlayerPos {
    pub pos: Vec3,        // final world position (terrain sets .y)
    pub altitude: f32,    // user-controlled offset (Q/E/scroll)
}
```

No `TerrainHeight` resource needed — terrain writes `pos.y` directly.

**System data flow:**
```
drone::fly  (or intro::run_intro during cinematic)
  reads:  keys, mouse, scroll
  writes: PlayerPos.pos.x/z (WASD), PlayerPos.altitude (Q/E/scroll)
  effect: sets drone transform from PlayerPos.pos

terrain::update_player_height
  reads:  HexGrid
  writes: PlayerPos.pos.y = interpolated_height + PlayerPos.altitude

terrain::track_active_hex
  reads:  PlayerPos, HexGrid, HexEntities
  writes: ActiveHex (terrain-owned, was CameraCell)

terrain::spawn_petals
  reads:  ActiveHex
```

**Ordering:** drone/intro write xz → terrain computes y → drone reads final pos.

`CameraCell` → renamed `ActiveHex`, defined in `terrain/entities.rs`.
`CameraAltitude` → absorbed into `PlayerPos.altitude`.

### Intro is drone-only

Intro directly manipulates the drone `Transform` (tilt up/down rotation). The
starting height is already set before intro runs — no terrain queries needed.
Intro depends only on `drone` (Player, DroneConfig).

Intro transitions `GameState::Intro → GameState::Running` when animation completes.
Drone and terrain systems are gated by `in_state(GameState::Running)`.

## Target Dependency Graph

```
        main (CLI, PlayerPos, GameState)
        / \
   drone   terrain
    |        |
  intro    math
```

- `main` depends on: nothing (defines PlayerPos, GameState, CLI)
- `drone` depends on: `main` (PlayerPos, GameState)
- `terrain` depends on: `main` (PlayerPos, GameState), `math`
- `intro` depends on: `drone` (Player, DroneConfig), `main` (GameState)
- `math` depends on: nothing

No cycles. No module imports more than two siblings.
`GameState` replaces both `IntroSequence` (state) and `InspectorActive` —
terrain no longer depends on intro at all.

## Summary of Moves

| Item | From | To |
|------|------|----|
| `GameState` enum | new | main.rs (Bevy `States`) |
| `PlayerPos` | new | main.rs (single shared resource) |
| `IntroSequence` | intro/entities | replaced by `GameState` + local `IntroTimer` |
| `InspectorActive` | main.rs | replaced by `GameState::Debugging` |
| `CameraCell` | camera/entities | terrain/entities (renamed `ActiveHex`) |
| `CameraAltitude` | camera/entities | absorbed into `PlayerPos.altitude` |
| `TerrainCamera` | camera/entities | drone/entities (renamed `Player`) |
| `CursorRecentered` | camera/entities | drone/entities |
| `TerrainHeight` | — | not needed (`PlayerPos.pos.y` written by terrain) |
| `HexGrid` | grid/entities | terrain/entities |
| `HexEntities` | petals/entities | terrain/entities |
| `HexSunDisc`, `HeightPole` | petals/entities | terrain/entities |
| `QuadLeaf`, `TriLeaf`, `PetalEdge` | petals/entities | terrain/entities |
| `DrawnCells` | petals/entities | terrain/entities |
| `interpolate_height` | camera/systems | terrain/systems |
| `generate_grid` | grid/systems | terrain/systems |
| `spawn_petals` + helpers | petals/systems | terrain/systems |
| `fade_nearby_poles` | grid/systems | terrain/systems |
| Camera3d + Hdr + Bloom spawn | visuals/systems | drone/systems |
| `ActiveNeonMaterials` | visuals/entities | removed (inline per-system) |
| `GridConfig` + `PetalsConfig` | grid.rs + petals.rs | merged into `TerrainConfig` (nested) |
| `CameraConfig` | camera.rs | `DroneConfig` in drone.rs |
| `CameraPlugin` | camera.rs | `DronePlugin` in drone.rs |
| `move_camera` | camera/systems | `fly` in drone/systems |
| `VisualsConfig` | visuals.rs | bloom → `DroneConfig`, clear_color → `TerrainConfig` |
| `RenderMode` | main.rs | removed (hardcode Full) |
| `AppConfig` | main.rs | removed (CLI only `--debug`) |
| camera.rs, camera/ | src/ | renamed → drone.rs, drone/ |
| grid.rs, grid/ | src/ | removed |
| petals.rs, petals/ | src/ | removed |
| visuals.rs, visuals/ | src/ | removed |
