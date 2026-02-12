//! Petal-based entity hierarchy for hex gap geometry.
//!
//! Replaces the flat edge/gap-face spawning with a parent–child model:
//! `HexSunDisc` → `QuadLeaf`/`TriLeaf` → `PetalEdge`, enabling future
//! reactive height updates via entity references.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, Hex, VertexDirection};

use crate::camera::CameraCell;
use crate::grid::HexGrid;
use crate::intro::IntroSequence;
use crate::math;
use crate::visuals::ActiveNeonMaterials;
use crate::{AppConfig, RenderMode};

/// Per-plugin configuration for petal spawning.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct PetalsConfig {
    /// Thickness of edge line cuboids.
    pub edge_thickness: f32,
    /// How many hex rings around the camera to reveal per cell transition.
    pub reveal_radius: u32,
}

impl Default for PetalsConfig {
    fn default() -> Self {
        Self {
            edge_thickness: 0.03,
            reveal_radius: 2,
        }
    }
}

/// Progressive petal spawning as the camera reveals new cells.
pub struct PetalsPlugin(pub PetalsConfig);

impl Plugin for PetalsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HexSunDisc>()
            .register_type::<QuadLeaf>()
            .register_type::<TriLeaf>()
            .register_type::<PetalEdge>()
            .register_type::<PetalsConfig>()
            .insert_resource(self.0.clone())
            .init_resource::<DrawnCells>()
            .add_systems(
                Update,
                (
                    draw_initial_cell,
                    spawn_cell_geometry.after(crate::camera::track_camera_cell),
                ),
            );
    }
}

/// Marker on hex face entities. Spawned by `grid.rs`.
#[derive(Component, Reflect)]
pub struct HexSunDisc {
    /// The hex coordinate this disc represents.
    #[reflect(ignore)]
    #[expect(dead_code, reason = "stored for future entity lookup and debugging")]
    pub hex: Hex,
}

/// Gap quad between two adjacent hexes. Child of the owning `HexSunDisc`.
#[derive(Component, Reflect)]
pub struct QuadLeaf {
    /// Even edge index on the owner hex (0, 2, or 4).
    pub edge_index: u8,
    /// Entity of the neighbor `HexSunDisc`.
    pub neighbor_disc: Entity,
}

/// Gap triangle at a 3-hex vertex junction. Child of the owning `HexSunDisc`.
#[derive(Component, Reflect)]
pub struct TriLeaf {
    /// Vertex index on the owner hex (0 or 1).
    pub vertex_index: u8,
    /// The other two `HexSunDisc` entities at this junction.
    pub neighbor_discs: [Entity; 2],
}

/// Edge cuboid mesh. Child of a `QuadLeaf`.
#[derive(Component, Reflect)]
struct PetalEdge;

/// Maps hex coordinates to their spawned `HexSunDisc` entity IDs.
#[derive(Resource)]
pub struct HexEntities {
    /// Lookup from hex to entity.
    pub map: HashMap<Hex, Entity>,
}

/// Tracks which hexes have already had their petals spawned.
#[derive(Resource, Default)]
struct DrawnCells {
    cells: HashSet<Hex>,
}

#[allow(clippy::too_many_arguments)]
fn draw_initial_cell(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    grid: Option<Res<HexGrid>>,
    hex_entities: Option<Res<HexEntities>>,
    neon: Res<ActiveNeonMaterials>,
    config: Res<AppConfig>,
    drawn: ResMut<DrawnCells>,
    intro: Res<IntroSequence>,
    petals_cfg: Res<PetalsConfig>,
    mut done: Local<bool>,
) {
    if *done || !intro.initial_draw_triggered {
        return;
    }
    *done = true;
    let Some(grid) = grid else { return };
    let Some(hex_entities) = hex_entities else {
        return;
    };
    spawn_geometry_for_cell(
        commands,
        meshes,
        &grid,
        &hex_entities,
        &neon,
        &config,
        drawn,
        &petals_cfg,
        Hex::ZERO,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_cell_geometry(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    grid: Option<Res<HexGrid>>,
    hex_entities: Option<Res<HexEntities>>,
    neon: Res<ActiveNeonMaterials>,
    config: Res<AppConfig>,
    cell: Res<CameraCell>,
    drawn: ResMut<DrawnCells>,
    petals_cfg: Res<PetalsConfig>,
) {
    if !cell.changed {
        return;
    }
    let Some(grid) = grid else { return };
    let Some(hex_entities) = hex_entities else {
        return;
    };
    spawn_geometry_for_cell(
        commands,
        meshes,
        &grid,
        &hex_entities,
        &neon,
        &config,
        drawn,
        &petals_cfg,
        cell.current,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_geometry_for_cell(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    grid: &HexGrid,
    hex_entities: &HexEntities,
    neon: &ActiveNeonMaterials,
    config: &AppConfig,
    mut drawn: ResMut<DrawnCells>,
    petals_cfg: &PetalsConfig,
    center: Hex,
) {
    let hexes_to_draw: Vec<Hex> = hexx::shapes::hexagon(center, petals_cfg.reveal_radius)
        .filter(|h| grid.heights.contains_key(h))
        .collect();

    for &hex in &hexes_to_draw {
        if drawn.cells.contains(&hex) {
            continue;
        }
        drawn.cells.insert(hex);

        let Some(&owner_entity) = hex_entities.map.get(&hex) else {
            continue;
        };

        spawn_petals_for_hex(
            &mut commands,
            &mut meshes,
            grid,
            hex_entities,
            neon,
            config,
            petals_cfg,
            hex,
            owner_entity,
        );
    }
}

/// Ownership convention:
/// - QuadLeafs at even edges: 0, 2, 4
/// - TriLeafs at vertices: 0 (even parity) and 1 (odd parity)
#[allow(clippy::too_many_arguments)]
fn spawn_petals_for_hex(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    grid: &HexGrid,
    hex_entities: &HexEntities,
    neon: &ActiveNeonMaterials,
    config: &AppConfig,
    petals_cfg: &PetalsConfig,
    hex: Hex,
    owner_entity: Entity,
) {
    // 3 QuadLeafs at even edges (0, 2, 4)
    for &edge_idx in &[0u8, 2, 4] {
        let dir = EdgeDirection::ALL_DIRECTIONS[edge_idx as usize];
        let neighbor = hex.neighbor(dir);

        if !grid.heights.contains_key(&neighbor) {
            continue;
        }
        let Some(&neighbor_entity) = hex_entities.map.get(&neighbor) else {
            continue;
        };

        spawn_quad_leaf(
            commands,
            meshes,
            grid,
            neon,
            config,
            petals_cfg,
            hex,
            edge_idx,
            neighbor,
            owner_entity,
            neighbor_entity,
        );
    }

    // 2 TriLeafs at vertices 0 and 1
    for &vtx_idx in &[0u8, 1] {
        spawn_tri_leaf(
            commands,
            meshes,
            grid,
            hex_entities,
            neon,
            config,
            hex,
            vtx_idx,
            owner_entity,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_quad_leaf(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    grid: &HexGrid,
    neon: &ActiveNeonMaterials,
    config: &AppConfig,
    petals_cfg: &PetalsConfig,
    hex: Hex,
    edge_index: u8,
    neighbor: Hex,
    owner_entity: Entity,
    neighbor_entity: Entity,
) {
    let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
    let vertex_dirs = dir.vertex_directions();
    let v0_idx = vertex_dirs[0].index();
    let v1_idx = vertex_dirs[1].index();

    let opp_dir = dir.const_neg();
    let opp_vertex_dirs = opp_dir.vertex_directions();
    // Swapped: facing vertices mirror
    let n0_idx = opp_vertex_dirs[1].index();
    let n1_idx = opp_vertex_dirs[0].index();

    let Some(&va0) = grid.vertex_positions.get(&(hex, v0_idx)) else {
        return;
    };
    let Some(&va1) = grid.vertex_positions.get(&(hex, v1_idx)) else {
        return;
    };
    let Some(&vb0) = grid.vertex_positions.get(&(neighbor, n0_idx)) else {
        return;
    };
    let Some(&vb1) = grid.vertex_positions.get(&(neighbor, n1_idx)) else {
        return;
    };

    let leaf_name = format!(
        "QuadLeaf({},{})e{}↔({},{})",
        hex.x, hex.y, edge_index, neighbor.x, neighbor.y
    );

    let inverse_tf = world_space_inverse(grid, hex);

    let leaf_entity = commands
        .spawn((
            QuadLeaf {
                edge_index,
                neighbor_disc: neighbor_entity,
            },
            Name::new(leaf_name),
            inverse_tf,
        ))
        .id();

    // Perimeter edges (along hex boundary)
    if matches!(config.render_mode, RenderMode::Perimeter | RenderMode::Full) {
        let edge_a = spawn_edge_line(commands, meshes, neon, petals_cfg, va0, va1);
        let edge_b = spawn_edge_line(commands, meshes, neon, petals_cfg, vb0, vb1);
        commands.entity(leaf_entity).add_children(&[edge_a, edge_b]);
    }

    // Cross-gap edges + quad face
    if matches!(config.render_mode, RenderMode::CrossGap | RenderMode::Full) {
        let cross_a = spawn_edge_line(commands, meshes, neon, petals_cfg, va0, vb0);
        let cross_b = spawn_edge_line(commands, meshes, neon, petals_cfg, va1, vb1);
        let face = spawn_quad_face(commands, meshes, neon, va0, va1, vb1, vb0);
        commands
            .entity(leaf_entity)
            .add_children(&[cross_a, cross_b, face]);
    }

    commands.entity(owner_entity).add_child(leaf_entity);
}

#[allow(clippy::too_many_arguments)]
fn spawn_tri_leaf(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    grid: &HexGrid,
    hex_entities: &HexEntities,
    neon: &ActiveNeonMaterials,
    config: &AppConfig,
    hex: Hex,
    vertex_index: u8,
    owner_entity: Entity,
) {
    // Only spawn in CrossGap or Full mode
    if !matches!(config.render_mode, RenderMode::CrossGap | RenderMode::Full) {
        return;
    }

    let dir = VertexDirection::ALL_DIRECTIONS[vertex_index as usize];
    let grid_vertex = hexx::GridVertex {
        origin: hex,
        direction: dir,
    };
    let coords = grid_vertex.coordinates();

    // All 3 hexes must exist
    if !coords.iter().all(|c| grid.heights.contains_key(c)) {
        return;
    }

    // Dedup: only the origin hex (coords[0]) spawns this triangle
    if coords[0] != hex {
        return;
    }

    let v_idx = dir.index();
    let Some(&v0) = grid.vertex_positions.get(&(coords[0], v_idx)) else {
        return;
    };
    let Some(v1) = find_equivalent_vertex(grid, coords[1], &grid_vertex) else {
        return;
    };
    let Some(v2) = find_equivalent_vertex(grid, coords[2], &grid_vertex) else {
        return;
    };

    // Look up neighbor entities
    let Some(&neighbor1_entity) = hex_entities.map.get(&coords[1]) else {
        return;
    };
    let Some(&neighbor2_entity) = hex_entities.map.get(&coords[2]) else {
        return;
    };

    let leaf_name = format!(
        "TriLeaf({},{})v{}↔({},{})↔({},{})",
        hex.x, hex.y, vertex_index, coords[1].x, coords[1].y, coords[2].x, coords[2].y
    );

    // TriLeaf has the face mesh on itself
    let face_mesh = build_tri_mesh(v0, v1, v2);
    let face_handle = meshes.add(face_mesh);

    let inverse_tf = world_space_inverse(grid, hex);

    let leaf_entity = commands
        .spawn((
            TriLeaf {
                vertex_index,
                neighbor_discs: [neighbor1_entity, neighbor2_entity],
            },
            Name::new(leaf_name),
            Mesh3d(face_handle),
            MeshMaterial3d(neon.gap_face_material.clone()),
            inverse_tf,
        ))
        .id();

    commands.entity(owner_entity).add_child(leaf_entity);
}

/// Computes a transform that cancels the parent HexSunDisc's translation + scale,
/// so children of this node can use world-space coordinates directly.
fn world_space_inverse(grid: &HexGrid, hex: Hex) -> Transform {
    let center_2d = grid.layout.hex_to_world_pos(hex);
    let height = grid.heights[&hex];
    let radius = grid.radii[&hex];

    let parent_t = Vec3::new(center_2d.x, height, center_2d.y);
    let parent_s = Vec3::new(radius, 1.0, radius);

    Transform {
        translation: Vec3::new(
            -parent_t.x / parent_s.x,
            -parent_t.y / parent_s.y,
            -parent_t.z / parent_s.z,
        ),
        scale: Vec3::new(1.0 / parent_s.x, 1.0 / parent_s.y, 1.0 / parent_s.z),
        ..default()
    }
}

fn find_equivalent_vertex(grid: &HexGrid, hex: Hex, target: &hexx::GridVertex) -> Option<Vec3> {
    for dir in VertexDirection::ALL_DIRECTIONS {
        let candidate = hexx::GridVertex {
            origin: hex,
            direction: dir,
        };
        if candidate.equivalent(target) {
            return grid.vertex_positions.get(&(hex, dir.index())).copied();
        }
    }
    None
}

/// Spawns an edge cuboid and returns its entity ID (no parent set).
fn spawn_edge_line(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    neon: &ActiveNeonMaterials,
    petals_cfg: &PetalsConfig,
    from: Vec3,
    to: Vec3,
) -> Entity {
    let midpoint = (from + to) / 2.0;
    let diff = to - from;
    let length = diff.length();

    let mesh = meshes.add(Cuboid::new(
        length,
        petals_cfg.edge_thickness,
        petals_cfg.edge_thickness,
    ));

    let direction = diff.normalize();
    let rotation = Quat::from_rotation_arc(Vec3::X, direction);

    commands
        .spawn((
            PetalEdge,
            Mesh3d(mesh),
            MeshMaterial3d(neon.edge_material.clone()),
            Transform::from_translation(midpoint).with_rotation(rotation),
        ))
        .id()
}

/// Spawns a quad face entity and returns its entity ID (no parent set).
fn spawn_quad_face(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    neon: &ActiveNeonMaterials,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    v3: Vec3,
) -> Entity {
    let positions = vec![v0.to_array(), v1.to_array(), v2.to_array(), v3.to_array()];
    let normal = math::compute_normal(v0, v1, v2);
    let normals = vec![normal.to_array(); 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0u16, 1, 2, 0, 2, 3];

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U16(indices));

    commands
        .spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(neon.gap_face_material.clone()),
        ))
        .id()
}

/// Builds a triangle mesh (no entity spawned).
fn build_tri_mesh(v0: Vec3, v1: Vec3, v2: Vec3) -> Mesh {
    let positions = vec![v0.to_array(), v1.to_array(), v2.to_array()];
    let normal = math::compute_normal(v0, v1, v2);
    let normals = vec![normal.to_array(); 3];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
    let indices = vec![0u16, 1, 2];

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U16(indices))
}
