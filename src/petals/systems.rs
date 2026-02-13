use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, Hex, VertexDirection};

use super::entities::{DrawnCells, HexCtx, PetalEdge, PetalRes, QuadLeaf, TriLeaf};
use crate::RenderMode;
use crate::camera::CameraCell;
use crate::grid::HexGrid;
use crate::intro::IntroSequence;
use crate::math;

// ── System ──────────────────────────────────────────────────────────

/// Single system: initial draw (intro trigger) + progressive reveal (camera cell changes).
pub fn spawn_petals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    res: PetalRes,
    cell: Res<CameraCell>,
    mut drawn: ResMut<DrawnCells>,
    intro: Res<IntroSequence>,
    mut intro_done: Local<bool>,
) {
    let center = if !*intro_done && intro.initial_draw_triggered {
        *intro_done = true;
        Some(Hex::ZERO)
    } else if cell.changed {
        Some(cell.current)
    } else {
        None
    };

    let Some(center) = center else { return };
    let Ok(grid) = res.grid_q.single() else {
        return;
    };

    for hex in hexx::shapes::hexagon(center, res.petals_cfg.reveal_radius) {
        if !grid.heights.contains_key(&hex) || drawn.cells.contains(&hex) {
            continue;
        }
        drawn.cells.insert(hex);

        let Some(&owner_entity) = res.hex_entities.map.get(&hex) else {
            continue;
        };

        let ctx = HexCtx {
            hex,
            owner_entity,
            inverse_tf: world_space_inverse(grid, hex),
        };

        for &edge_idx in &[0u8, 2, 4] {
            spawn_quad_leaf(&mut commands, &mut meshes, &res, grid, &ctx, edge_idx);
        }
        for &vtx_idx in &[0u8, 1] {
            spawn_tri_leaf(&mut commands, &mut meshes, &res, grid, &ctx, vtx_idx);
        }
    }
}

// ── Leaf spawn helpers ──────────────────────────────────────────────

fn spawn_quad_leaf(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    res: &PetalRes,
    grid: &HexGrid,
    ctx: &HexCtx,
    edge_index: u8,
) -> Option<()> {
    let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
    let neighbor = ctx.hex.neighbor(dir);

    grid.heights.get(&neighbor)?;
    let &neighbor_entity = res.hex_entities.map.get(&neighbor)?;

    let vertex_dirs = dir.vertex_directions();
    let v0_idx = vertex_dirs[0].index();
    let v1_idx = vertex_dirs[1].index();

    let opp_dir = dir.const_neg();
    let opp_vertex_dirs = opp_dir.vertex_directions();
    let n0_idx = opp_vertex_dirs[1].index();
    let n1_idx = opp_vertex_dirs[0].index();

    let &va0 = grid.vertex_positions.get(&(ctx.hex, v0_idx))?;
    let &va1 = grid.vertex_positions.get(&(ctx.hex, v1_idx))?;
    let &vb0 = grid.vertex_positions.get(&(neighbor, n0_idx))?;
    let &vb1 = grid.vertex_positions.get(&(neighbor, n1_idx))?;

    let leaf_name = format!(
        "QuadLeaf({},{})e{}↔({},{})",
        ctx.hex.x, ctx.hex.y, edge_index, neighbor.x, neighbor.y
    );

    let leaf_entity = commands
        .spawn((
            QuadLeaf {
                edge_index,
                neighbor_disc: neighbor_entity,
            },
            Name::new(leaf_name),
            Visibility::default(),
            ctx.inverse_tf,
        ))
        .id();

    // Perimeter edges (along hex boundary)
    if matches!(
        res.config.render_mode,
        RenderMode::Perimeter | RenderMode::Full
    ) {
        let edge_a = spawn_edge_line(commands, meshes, res, va0, va1);
        let edge_b = spawn_edge_line(commands, meshes, res, vb0, vb1);
        commands.entity(leaf_entity).add_children(&[edge_a, edge_b]);
    }

    // Cross-gap edges + quad face
    if matches!(
        res.config.render_mode,
        RenderMode::CrossGap | RenderMode::Full
    ) {
        let cross_a = spawn_edge_line(commands, meshes, res, va0, vb0);
        let cross_b = spawn_edge_line(commands, meshes, res, va1, vb1);
        let face = spawn_quad_face(commands, meshes, res, va0, va1, vb1, vb0);
        commands
            .entity(leaf_entity)
            .add_children(&[cross_a, cross_b, face]);
    }

    commands.entity(ctx.owner_entity).add_child(leaf_entity);
    Some(())
}

fn spawn_tri_leaf(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    res: &PetalRes,
    grid: &HexGrid,
    ctx: &HexCtx,
    vertex_index: u8,
) -> Option<()> {
    if !matches!(
        res.config.render_mode,
        RenderMode::CrossGap | RenderMode::Full
    ) {
        return None;
    }

    let dir = VertexDirection::ALL_DIRECTIONS[vertex_index as usize];
    let grid_vertex = hexx::GridVertex {
        origin: ctx.hex,
        direction: dir,
    };
    let coords = grid_vertex.coordinates();

    coords
        .iter()
        .all(|c| grid.heights.contains_key(c))
        .then_some(())?;
    (coords[0] == ctx.hex).then_some(())?;

    let v_idx = dir.index();
    let &v0 = grid.vertex_positions.get(&(coords[0], v_idx))?;
    let v1 = find_equivalent_vertex(grid, coords[1], &grid_vertex)?;
    let v2 = find_equivalent_vertex(grid, coords[2], &grid_vertex)?;

    let &neighbor1_entity = res.hex_entities.map.get(&coords[1])?;
    let &neighbor2_entity = res.hex_entities.map.get(&coords[2])?;

    let leaf_name = format!(
        "TriLeaf({},{})v{}↔({},{})↔({},{})",
        ctx.hex.x, ctx.hex.y, vertex_index, coords[1].x, coords[1].y, coords[2].x, coords[2].y
    );

    let face_handle = meshes.add(build_tri_mesh(v0, v1, v2));

    let leaf_entity = commands
        .spawn((
            TriLeaf {
                vertex_index,
                neighbor_discs: [neighbor1_entity, neighbor2_entity],
            },
            Name::new(leaf_name),
            Mesh3d(face_handle),
            MeshMaterial3d(res.neon.gap_face_material.clone()),
            ctx.inverse_tf,
        ))
        .id();

    commands.entity(ctx.owner_entity).add_child(leaf_entity);
    Some(())
}

// ── Mesh spawn helpers ──────────────────────────────────────────────

fn spawn_edge_line(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    res: &PetalRes,
    from: Vec3,
    to: Vec3,
) -> Entity {
    let midpoint = (from + to) / 2.0;
    let diff = to - from;
    let length = diff.length();

    let mesh = meshes.add(Cuboid::new(
        length,
        res.petals_cfg.edge_thickness,
        res.petals_cfg.edge_thickness,
    ));

    let direction = diff.normalize();
    let rotation = Quat::from_rotation_arc(Vec3::X, direction);

    commands
        .spawn((
            PetalEdge,
            Mesh3d(mesh),
            MeshMaterial3d(res.neon.edge_material.clone()),
            Transform::from_translation(midpoint).with_rotation(rotation),
        ))
        .id()
}

fn spawn_quad_face(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    res: &PetalRes,
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
            MeshMaterial3d(res.neon.gap_face_material.clone()),
        ))
        .id()
}

// ── Pure helpers ────────────────────────────────────────────────────

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
