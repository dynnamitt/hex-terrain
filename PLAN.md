# Plan: Hex Terrain Viewer (Bevy 0.18)

## Context

New Bevy ECS project in ~/CODE. Renders a hexagonal grid of noise-derived terrain points with neon edge lighting. Each grid point is a hex with 6 vertices ("virtual points"). The spaces between hexes form triangles (at 3-hex junctions) and rectangles (between 2-hex edge pairs). Camera moves horizontally, revealing geometry progressively. Minimalist/neon aesthetic.

## Dependencies

| Crate | Version | Why |
|---|---|---|
| `bevy` | 0.18 | Latest stable. Selective features (see hexx pattern). |
| `hexx` | path `../hexx` | Local lib. Hex math, layout, mesh builders, shapes. `features = ["bevy"]` |
| `noise` | 0.9 | `Fbm<Perlin>` for terrain height. |
| `clap` | 4 | CLI arg for render mode selection. |
| `bevy-inspector-egui` | 0.36 | Entity/resource inspection during dev. |
| `bevy_egui` | 0.39 | Required by inspector. |

### MCP Debugger (external tool, not a Cargo dep)

| Tool | Install | Purpose |
|---|---|---|
| `bevy_debugger_mcp` | `cargo install bevy_debugger_mcp` | AI-assisted debugging via Claude Code MCP |

**Game-side setup**: Add `RemotePlugin::default()` to the app. Enable `bevy_remote` feature in Bevy deps.

**Claude Code MCP config** (`~/.claude/claude_code_config.json`):
```json
{
  "mcpServers": {
    "bevy-debugger": {
      "command": "bevy-debugger-mcp",
      "args": ["--stdio"],
      "type": "stdio",
      "env": { "BEVY_BRP_HOST": "localhost", "BEVY_BRP_PORT": "15702" }
    }
  }
}
```

> **Version note**: `bevy_debugger_mcp` v0.1.8 was built against Bevy 0.16. Communicates via BRP WebSocket, so should work with 0.18's `RemotePlugin`. If protocol issues surface, we'll address them.

**Not using** a camera plugin — movement rules are too specific. Custom system is simpler.

## Project Structure

```
~/CODE/hex-terrain/
  Cargo.toml
  src/
    main.rs        # App entry, CLI args, plugin registration, RemotePlugin
    grid.rs        # Point/vertex spawning, HexGrid resource, noise heights
    camera.rs      # TerrainCamera, WASD movement, vertex-based height interpolation
    edges.rs       # Progressive edge/face spawning, render mode logic
    visuals.rs     # Bloom/camera setup, NeonMaterials, background
```

## Core Geometry Model

Each **Point** in the grid is a hex center. Each hex center produces **6 vertices** at 1m radius (the corners of the rendered hexagon). These vertices are the building blocks for ALL geometry:

```
          v2----v1          <- hex A vertices
         /   A    \
       v3          v0       Each hex has 6 vertices (v0-v5)
         \        /         spaced 1m from center
          v4----v5

     4m gap between hex centers

          v2----v1
         /   B    \         <- neighboring hex B
       v3          v0
         \        /
          v4----v5
```

Between adjacent hexes, the geometry forms:
- **Rectangles**: 2 vertices from hex A + 2 facing vertices from hex B (between parallel hex edges)
- **Triangles**: 1 vertex from each of 3 adjacent hexes (at triple-hex junctions)

All three face types (hex, rectangle, triangle) are **filled surfaces** with neon edges on top.

### Vertex Heights — Trait-Based Strategy

Height calculation is pluggable via a trait, selected by CLI arg `--height-mode`:

```rust
trait VertexHeightStrategy {
    fn compute(&self, parent_height: f32, neighbor_heights: &[f32]) -> f32;
}

#[derive(Clone, Copy, clap::ValueEnum, Default)]
enum HeightMode {
    /// Smooth: average of parent + neighboring hex center heights
    #[default]
    Smooth,
    /// Blocky: all vertices inherit parent hex center height (flat-top hexes)
    Blocky,
}
```

**Smooth mode** (default):
- A vertex at the junction of hex A, B, C gets: `(h_A + h_B + h_C) / 3.0`
- A vertex at the junction of hex A, B (grid edge): `(h_A + h_B) / 2.0`
- A vertex with only its parent hex (corner of grid): `h_A`
- Creates smooth terrain slopes across the grid

**Blocky mode**:
- All 6 vertices of a hex get the same height as the hex center
- Creates plateaus with sharp drops at gap edges (Minecraft-like steps)
- Gap rectangles and triangles become sloped ramps between flat hexes

## CLI Render Modes

```rust
#[derive(Clone, Copy, clap::ValueEnum)]
enum RenderMode {
    /// Mode 1: Only hex perimeter edges (6 edges per hex)
    Perimeter,
    /// Mode 2: Only cross-gap edges (vertex-to-vertex between hexes)
    CrossGap,
    /// Mode 3: Both perimeter + cross-gap (full tessellation)
    Full,
}
```

Usage: `cargo run -- --mode full --height-mode smooth` (both default)

## ECS Architecture

### Components

```rust
// grid.rs
#[derive(Component)]
struct Point { hex: Hex, height: f32 }

#[derive(Component)]
struct HexVertex { parent_hex: Hex, vertex_index: u8, height: f32 }

// edges.rs
#[derive(Component)]
struct EdgeLine;       // marker for neon edge line entities

#[derive(Component)]
struct GapFace;        // marker for filled triangle/rectangle face entities

// camera.rs
#[derive(Component)]
struct TerrainCamera;
```

### Resources

```rust
// grid.rs
#[derive(Resource)]
struct HexGrid {
    layout: HexLayout,                             // pointy, scale=Vec2::splat(4.0)
    render_layout: HexLayout,                      // pointy, scale=Vec2::splat(1.0) for vertex positions
    heights: HashMap<Hex, f32>,                    // hex center heights (from noise)
    vertex_positions: HashMap<(Hex, u8), Vec3>,    // (hex, vertex_idx) -> world position with derived height
}

// camera.rs
#[derive(Resource, Default)]
struct CameraCell { current: Hex, previous: Option<Hex> }

// edges.rs
#[derive(Resource, Default)]
struct DrawnCells { cells: HashSet<Hex> }

// visuals.rs
#[derive(Resource)]
struct NeonMaterials {
    edge_material: Handle<StandardMaterial>,   // bright emissive for neon edges
    hex_face_material: Handle<StandardMaterial>,  // dark subtle for hex faces
    gap_face_material: Handle<StandardMaterial>,  // dark subtle for triangle/rect faces
}

// main.rs
#[derive(Resource)]
struct AppConfig { render_mode: RenderMode, height_mode: HeightMode }
```

### System Schedule

**Startup** (chained):
1. `setup_visuals` — camera (Camera3d + Bloom + Tonemapping), NeonMaterials, clear color
2. `generate_grid` — spawn Points, compute vertex heights, insert HexGrid
3. `spawn_hex_faces` — spawn filled hex plane meshes for all points (always visible)
4. `draw_initial_cell` — edges + gap faces for starting camera cell

**Update** (chained):
1. `move_camera` — WASD movement, mouse yaw, vertex-based height interpolation + 2m
2. `track_camera_cell` — XZ -> hex via `world_pos_to_hex()`, update CameraCell
3. `spawn_cell_geometry` — on cell change: spawn edges + filled gap faces per render mode

## Key Implementation Details

### Grid Generation (`grid.rs`)

- Standard pointy-top hex grid (hexx default)
- `shapes::hexagon(Hex::ZERO, GRID_RADIUS)` for hex coordinates
- `HexLayout { scale: Vec2::splat(4.0), ..default() }` for 4m spacing
- `HexLayout { scale: Vec2::splat(1.0), ..default() }` for vertex offset calculation
- Height: `Fbm::<Perlin>::new(seed).get([x/50.0, z/50.0])` -> `[0, MAX_HEIGHT]`
- Map `Vec2` from `hex_to_world_pos` to 3D: `pos.x -> X, pos.y -> Z`
- Mesh pattern from `hexx/examples/3d_columns.rs:127-140`

**Vertex position computation**: For each hex, compute 6 vertex world positions:
```rust
let center_2d = grid_layout.hex_to_world_pos(hex);
let center_3d = Vec3::new(center_2d.x, center_height, center_2d.y);
for i in 0..6 {
    let offset_2d = render_layout.hex_to_world_pos(Hex::ZERO); // vertex offset from center
    // Use hexx vertex/corner positions relative to hex center
    let vertex_world_xz = Vec2::new(center_2d.x + offset.x, center_2d.y + offset.y);
    let vertex_height = average_of_neighboring_hex_heights(hex, i, &heights);
    vertex_positions.insert((hex, i), Vec3::new(vertex_world_xz.x, vertex_height, vertex_world_xz.y));
}
```

### Constants

```rust
const GRID_RADIUS: u32 = 20;           // ~1200 hexes
const POINT_SPACING: f32 = 4.0;        // 4m between hex centers
const HEX_RENDER_SIZE: f32 = 1.0;      // 1m visual hex radius
const MAX_HEIGHT: f32 = 10.0;          // max terrain elevation
const CAMERA_HEIGHT_OFFSET: f32 = 2.0; // 2m above terrain
```

### Camera Height Interpolation (`camera.rs`)

Camera doesn't use hex center heights — it uses **vertex heights**:
1. Find the closest vertices to the camera XZ position (from `HexGrid.vertex_positions`)
2. Inverse-distance weight the nearest 3-4 vertices
3. Camera Y = weighted height + `CAMERA_HEIGHT_OFFSET`

This gives smooth height transitions across hex faces, gap rectangles, and gap triangles.

### Edge & Face Rendering (`edges.rs`)

**Render Mode 1 (Perimeter)**: For each hex in the revealed cell + neighbors:
- 6 neon edge lines along hex perimeter (v0->v1, v1->v2, ..., v5->v0)
- 6 filled hex face triangles (center + vertex pairs)

**Render Mode 2 (CrossGap)**: For each pair of adjacent hexes around the camera:
- Neon edge lines connecting facing vertices across the 4m gap
- Filled rectangle faces (4 vertices: 2 from each hex)
- Filled triangle faces at triple-hex junctions (3 vertices from 3 hexes)

**Render Mode 3 (Full)**: Both of the above — complete tessellation.

**Edge lines**: Thin `Cuboid` mesh, stretched via `Transform::scale.x`, highly emissive unlit material.

**Filled faces**: Custom triangle/quad meshes, dark subtle material with slight emissive tint.

**Progressive reveal**: `DrawnCells` tracks which cells have been revealed. On cell change, spawn geometry for the new cell's neighborhood.

### Neon Visuals (`visuals.rs`)

- `Bloom { intensity: 0.3, composite_mode: BloomCompositeMode::Additive, ..Bloom::NATURAL }`
- `Tonemapping::TonyMcMapface`
- Clear color: `Color::srgb(0.01, 0.01, 0.02)` (near-black)
- Edge material: `emissive: LinearRgba::rgb(0.0, 20.0, 40.0)`, `unlit: true` (bright cyan neon)
- Face materials: `base_color: dark`, `emissive: LinearRgba::rgb(0.05, 0.1, 0.15)` (subtle blue)

### Debug UI

- `EguiPlugin` + `bevy_inspector_egui::DefaultInspectorConfigPlugin` (from `heightmap_builder.rs:41-42`)
- Side panel: camera pos, current hex, render mode, height mode, grid stats, bloom sliders

## Implementation Sequence

1. **Scaffold** — `cargo init`, Cargo.toml, `main.rs` with DefaultPlugins + RemotePlugin + clap args
2. **visuals.rs** — camera + bloom + clear color. Verify: dark screen with bloom
3. **grid.rs** — generate grid, compute vertex heights, spawn hex face meshes. Verify: hex tiles visible
4. **camera.rs** — WASD + mouse yaw + vertex-height interpolation. Verify: smooth movement over terrain
5. **edges.rs** — progressive edge/face spawning per render mode. Verify: neon geometry appears on cell entry
6. **Polish** — tune bloom, emissive values, edge thickness. Add debug UI. Test all 3 render modes + both height modes

## Bevy Feature Selection

Mirror hexx's dev-dependency pattern (`hexx/Cargo.toml:78-133`), adding `bevy_post_process` for bloom, `"hdr"` for HDR, and `"bevy_remote"` for BRP debugger. Drop `bevy_picking`, `bevy_sprite*`, `bevy_text`, `bevy_ui*`.

## Bevy 0.18 API Notes

These were discovered during implementation and differ from earlier Bevy versions:

- **HDR**: `Camera { hdr: true }` is gone. Use the `Hdr` component (`bevy::render::view::Hdr`).
- **Bloom**: Moved from `bevy::core_pipeline::bloom` to `bevy::post_process::bloom`.
- **Events**: `EventReader<T>` replaced by `MessageReader<T>` (in prelude).
- **System chaining**: `.chain()` still works on tuples of systems.

## Reference Files

- `~/CODE/hexx/examples/3d_columns.rs` — Bevy 0.18 hex grid spawn pattern (Mesh3d, MeshMaterial3d, HexLayout, shapes::hexagon)
- `~/CODE/hexx/examples/heightmap_builder.rs` — egui inspector integration, HeightMapMeshBuilder, compute_mesh helper
- `~/CODE/hexx/src/layout.rs` — `hex_to_world_pos`, `world_pos_to_hex` API (Vec2 output)
- `~/CODE/hexx/src/mesh/plane_builder.rs` — PlaneMeshBuilder for hex plane meshes
- `~/CODE/hexx/Cargo.toml` — verified dep versions: Bevy 0.18, bevy-inspector-egui 0.36, bevy_egui 0.39

## Setup Steps (before implementation)

1. `cargo install bevy_debugger_mcp` — install MCP debugger CLI
2. Add MCP config to `~/.claude/claude_code_config.json` (see config above)
3. Verify: after `cargo run`, the MCP debugger can connect and query entities

## Verification

1. `cargo build` — compiles without warnings
2. `cargo run -- --mode full` — window opens, dark background, hex faces visible
3. WASD moves camera, Y smoothly interpolates across vertex heights + 2m
4. Edges + gap faces appear with neon glow as camera enters new cells
5. Previously revealed geometry persists
6. Bloom visible on edges (cyan glow halos), faces subtly lit
7. `cargo run -- --mode perimeter` — only hex outlines, no cross-gap edges
8. `cargo run -- --mode cross-gap` — only cross-gap edges, no perimeter
9. `cargo run -- --height-mode blocky` — flat hexes with stepped terrain
10. `cargo run -- --height-mode smooth` — interpolated vertex heights (default)
11. `RemotePlugin` active — bevy_debugger_mcp can observe entities and query state
