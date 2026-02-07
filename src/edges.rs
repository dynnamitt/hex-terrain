use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, Hex, VertexDirection};

use crate::camera::CameraCell;
use crate::grid::HexGrid;
use crate::visuals::NeonMaterials;
use crate::{AppConfig, RenderMode};

pub struct EdgesPlugin;

impl Plugin for EdgesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawnCells>()
            .add_systems(Startup, draw_initial_cell.after(crate::grid::generate_grid))
            .add_systems(Update, spawn_cell_geometry.after(crate::camera::track_camera_cell));
    }
}

#[derive(Component)]
struct EdgeLine;

#[derive(Component)]
struct GapFace;

#[derive(Resource, Default)]
struct DrawnCells {
    cells: HashSet<Hex>,
}

const EDGE_THICKNESS: f32 = 0.03;
const REVEAL_RADIUS: u32 = 2;

fn draw_initial_cell(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    grid: Option<Res<HexGrid>>,
    neon: Res<NeonMaterials>,
    config: Res<AppConfig>,
    drawn: ResMut<DrawnCells>,
) {
    let Some(grid) = grid else { return };
    spawn_geometry_for_cell(
        commands,
        meshes,
        &grid,
        &neon,
        &config,
        drawn,
        Hex::ZERO,
    );
}

fn spawn_cell_geometry(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    grid: Option<Res<HexGrid>>,
    neon: Res<NeonMaterials>,
    config: Res<AppConfig>,
    cell: Res<CameraCell>,
    drawn: ResMut<DrawnCells>,
) {
    if !cell.changed {
        return;
    }
    let Some(grid) = grid else { return };
    spawn_geometry_for_cell(
        commands,
        meshes,
        &grid,
        &neon,
        &config,
        drawn,
        cell.current,
    );
}

fn spawn_geometry_for_cell(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    grid: &HexGrid,
    neon: &NeonMaterials,
    config: &AppConfig,
    mut drawn: ResMut<DrawnCells>,
    center: Hex,
) {
    let hexes_to_draw: Vec<Hex> = hexx::shapes::hexagon(center, REVEAL_RADIUS)
        .filter(|h| grid.heights.contains_key(h))
        .collect();

    for &hex in &hexes_to_draw {
        if drawn.cells.contains(&hex) {
            continue;
        }
        drawn.cells.insert(hex);

        // Perimeter edges
        if matches!(config.render_mode, RenderMode::Perimeter | RenderMode::Full) {
            spawn_perimeter_edges(&mut commands, &mut meshes, grid, neon, hex);
        }

        // Cross-gap edges and faces
        if matches!(config.render_mode, RenderMode::CrossGap | RenderMode::Full) {
            spawn_cross_gap_geometry(&mut commands, &mut meshes, grid, neon, hex);
        }
    }
}

fn spawn_perimeter_edges(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    grid: &HexGrid,
    neon: &NeonMaterials,
    hex: Hex,
) {
    // 6 edges around the hex perimeter
    for i in 0..6u8 {
        let next = (i + 1) % 6;
        let Some(&v_a) = grid.vertex_positions.get(&(hex, i)) else {
            continue;
        };
        let Some(&v_b) = grid.vertex_positions.get(&(hex, next)) else {
            continue;
        };
        spawn_edge_line(commands, meshes, neon, v_a, v_b);
    }
}

fn spawn_cross_gap_geometry(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    grid: &HexGrid,
    neon: &NeonMaterials,
    hex: Hex,
) {
    // For each edge direction, connect facing vertices to neighbor hex
    for dir in &EdgeDirection::ALL_DIRECTIONS {
        let neighbor = hex.neighbor(*dir);
        if !grid.heights.contains_key(&neighbor) {
            continue;
        }

        // The two vertex indices of this hex on this edge side
        // For pointy-top: edge direction i corresponds to vertices between corners
        // EdgeDirection i has vertex_directions which give the two corner vertices
        let vertex_dirs = dir.vertex_directions();
        let v0_idx = vertex_dirs[0].index() as u8;
        let v1_idx = vertex_dirs[1].index() as u8;

        // Find the facing vertices on the neighbor hex
        // The opposite edge direction on the neighbor
        let opp_dir = dir.const_neg();
        let opp_vertex_dirs = opp_dir.vertex_directions();
        // The facing vertices are swapped (cw becomes ccw from other side)
        let n0_idx = opp_vertex_dirs[1].index() as u8;
        let n1_idx = opp_vertex_dirs[0].index() as u8;

        let Some(&va0) = grid.vertex_positions.get(&(hex, v0_idx)) else {
            continue;
        };
        let Some(&va1) = grid.vertex_positions.get(&(hex, v1_idx)) else {
            continue;
        };
        let Some(&vb0) = grid.vertex_positions.get(&(neighbor, n0_idx)) else {
            continue;
        };
        let Some(&vb1) = grid.vertex_positions.get(&(neighbor, n1_idx)) else {
            continue;
        };

        // Cross-gap edge lines (connecting facing vertices)
        spawn_edge_line(commands, meshes, neon, va0, vb0);
        spawn_edge_line(commands, meshes, neon, va1, vb1);

        // Rectangle face between the 4 vertices
        spawn_quad_face(commands, meshes, neon, va0, va1, vb1, vb0);
    }

    // Triangle faces at triple-hex junctions (vertex directions)
    for dir in VertexDirection::ALL_DIRECTIONS {
        let grid_vertex = hexx::GridVertex {
            origin: hex,
            direction: dir,
        };
        let coords = grid_vertex.coordinates();

        // Only spawn if all 3 hexes exist and this hex is the "origin" (avoid duplicates)
        let all_exist = coords.iter().all(|c| grid.heights.contains_key(c));
        if !all_exist {
            continue;
        }

        // Only the origin hex spawns the triangle (dedup)
        if coords[0] != hex {
            continue;
        }

        // Get one vertex from each of the 3 hexes at this junction
        let v_idx = dir.index() as u8;

        // The vertex on hex at direction `dir`
        let Some(&v0) = grid.vertex_positions.get(&(coords[0], v_idx)) else {
            continue;
        };

        // For the other two hexes, find which vertex index corresponds to this junction
        // coords[1] and coords[2] are the neighbors; find their vertex that is equivalent
        let Some(v1) = find_equivalent_vertex(grid, coords[1], &grid_vertex) else {
            continue;
        };
        let Some(v2) = find_equivalent_vertex(grid, coords[2], &grid_vertex) else {
            continue;
        };

        // Triangle edge lines
        spawn_edge_line(commands, meshes, neon, v0, v1);
        spawn_edge_line(commands, meshes, neon, v1, v2);
        spawn_edge_line(commands, meshes, neon, v2, v0);

        // Triangle face
        spawn_tri_face(commands, meshes, neon, v0, v1, v2);
    }
}

fn find_equivalent_vertex(
    grid: &HexGrid,
    hex: Hex,
    target: &hexx::GridVertex,
) -> Option<Vec3> {
    for dir in VertexDirection::ALL_DIRECTIONS {
        let candidate = hexx::GridVertex {
            origin: hex,
            direction: dir,
        };
        if candidate.equivalent(target) {
            return grid.vertex_positions.get(&(hex, dir.index() as u8)).copied();
        }
    }
    None
}

fn spawn_edge_line(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    neon: &NeonMaterials,
    from: Vec3,
    to: Vec3,
) {
    let midpoint = (from + to) / 2.0;
    let diff = to - from;
    let length = diff.length();
    if length < 0.001 {
        return;
    }

    // Create a thin cuboid stretched along the edge
    let mesh = meshes.add(Cuboid::new(length, EDGE_THICKNESS, EDGE_THICKNESS));

    // Rotation to align the cuboid's X axis with the edge direction
    let direction = diff.normalize();
    let rotation = Quat::from_rotation_arc(Vec3::X, direction);

    commands.spawn((
        EdgeLine,
        Mesh3d(mesh),
        MeshMaterial3d(neon.edge_material.clone()),
        Transform::from_translation(midpoint).with_rotation(rotation),
    ));
}

fn spawn_quad_face(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    neon: &NeonMaterials,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    v3: Vec3,
) {
    // Two triangles: v0-v1-v2 and v0-v2-v3
    let positions = vec![
        v0.to_array(),
        v1.to_array(),
        v2.to_array(),
        v3.to_array(),
    ];

    let normal = compute_normal(v0, v1, v2);
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

    commands.spawn((
        GapFace,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(neon.gap_face_material.clone()),
    ));
}

fn spawn_tri_face(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    neon: &NeonMaterials,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) {
    let positions = vec![v0.to_array(), v1.to_array(), v2.to_array()];
    let normal = compute_normal(v0, v1, v2);
    let normals = vec![normal.to_array(); 3];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
    let indices = vec![0u16, 1, 2];

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U16(indices));

    commands.spawn((
        GapFace,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(neon.gap_face_material.clone()),
    ));
}

fn compute_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    edge1.cross(edge2).normalize_or_zero()
}

