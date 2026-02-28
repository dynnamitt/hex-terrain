//! Startup systems for height-based terrain.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, Hex, VertexDirection, shapes};

use super::HTerrainConfig;
use super::entities::{
    Corner, HCell, HGrid, Quad, QuadOwner, QuadPos2Emitter, QuadPos3Emitter, QuadTail, Tri,
    TriOwner, TriPos1Emitter, TriPos2Emitter,
};
use super::h_grid_layout::HGridLayout;
use crate::DebugFlag;
use crate::math;

/// Spawns the [`HGrid`] entity with [`HCell`] children, [`Corner`] grandchildren,
/// and Quad/Tri gap geometry with distributed emitter markers.
pub fn generate_h_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<HTerrainConfig>,
    debug: Res<DebugFlag>,
) {
    let g = &cfg.grid;
    let terrain = HGridLayout::from_settings(g);

    let edge_thickness = 0.02;
    let debug_assets = debug.0.then(|| {
        let sphere_mesh = meshes.add(Sphere::new(0.08));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.2, 0.8),
            emissive: LinearRgba::rgb(4.0, 0.8, 3.2),
            unlit: true,
            ..default()
        });
        (sphere_mesh, material)
    });

    let gap_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.03, 0.05),
        emissive: LinearRgba::rgb(0.03, 0.06, 0.1),
        cull_mode: None,
        ..default()
    });

    let grid_entity = commands
        .spawn((
            Name::new("HGrid"),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    // ── Pass 1: Spawn HCells + Corners, build lookup map ─────────
    let mut corner_entities: HashMap<(Hex, u8), Entity> = HashMap::new();

    for hex in shapes::hexagon(Hex::ZERO, g.radius) {
        let center = terrain.hex_to_world_pos(hex);
        let height = terrain.height(&hex).unwrap();
        let radius = terrain.radius(&hex).unwrap();

        let cell_entity = commands
            .spawn((
                HCell { hex },
                Name::new(format!("HCell({},{})", hex.x, hex.y)),
                Transform::from_xyz(center.x, height, center.y),
                Visibility::default(),
            ))
            .id();

        for i in 0..6usize {
            let uc = terrain.unit_corner(i);
            let local_offset = Vec3::new(uc.x * radius, 0.0, uc.y * radius);
            let mut corner = commands.spawn((
                Corner { index: i as u8 },
                Name::new(format!("Corner({},{})v{}", hex.x, hex.y, i)),
                Transform::from_translation(local_offset),
                Visibility::default(),
            ));
            if let Some((ref sphere_mesh, ref material)) = debug_assets {
                corner.insert((
                    Mesh3d(sphere_mesh.clone()),
                    MeshMaterial3d(material.clone()),
                ));

                let uc_next = terrain.unit_corner((i + 1) % 6);
                let next_offset = Vec3::new(uc_next.x * radius, 0.0, uc_next.y * radius);
                let edge_vec = next_offset - local_offset;
                let length = edge_vec.length();
                let midpoint = edge_vec / 2.0;
                let rotation = Quat::from_rotation_arc(Vec3::X, edge_vec.normalize());
                let edge_mesh = meshes.add(Cuboid::new(length, edge_thickness, edge_thickness));

                corner.with_child((
                    Mesh3d(edge_mesh),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(midpoint).with_rotation(rotation),
                ));
            }
            let corner_entity = corner.id();
            corner_entities.insert((hex, i as u8), corner_entity);
            commands.entity(cell_entity).add_child(corner_entity);
        }

        commands.entity(grid_entity).add_child(cell_entity);
    }

    // ── Pass 2: Spawn Quad and Tri gap geometry with markers ─────
    for hex in shapes::hexagon(Hex::ZERO, g.radius) {
        // Quads: even edge indices 0, 2, 4
        for edge_index in [0u8, 2, 4] {
            spawn_quad(
                &mut commands,
                &mut meshes,
                &gap_material,
                &terrain,
                &corner_entities,
                hex,
                edge_index,
            );
        }

        // Tris: vertex indices 0, 1
        for vertex_index in [0u8, 1] {
            spawn_tri(
                &mut commands,
                &mut meshes,
                &gap_material,
                &terrain,
                &corner_entities,
                hex,
                vertex_index,
            );
        }
    }

    commands.entity(grid_entity).insert(HGrid { terrain });
}

// ── Quad gap spawning ────────────────────────────────────────────

fn spawn_quad(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    gap_material: &Handle<StandardMaterial>,
    terrain: &HGridLayout,
    corner_entities: &HashMap<(Hex, u8), Entity>,
    hex: Hex,
    edge_index: u8,
) -> Option<()> {
    let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
    let neighbor = hex.neighbor(dir);

    let vertex_dirs = dir.vertex_directions();
    let v0_idx = vertex_dirs[0].index();
    let v1_idx = vertex_dirs[1].index();

    let opp_dir = dir.const_neg();
    let opp_vertex_dirs = opp_dir.vertex_directions();
    let n0_idx = opp_vertex_dirs[1].index();
    let n1_idx = opp_vertex_dirs[0].index();

    // All 4 corner entities must exist (grid-edge guard)
    let &owner_entity = corner_entities.get(&(hex, v0_idx))?;
    let &tail_entity = corner_entities.get(&(hex, v1_idx))?;
    let &pos2_entity = corner_entities.get(&(neighbor, n0_idx))?;
    let &pos3_entity = corner_entities.get(&(neighbor, n1_idx))?;

    // All 4 vertex positions
    let v0 = terrain.vertex(hex, v0_idx)?;
    let v1 = terrain.vertex(neighbor, n0_idx)?;
    let v2 = terrain.vertex(neighbor, n1_idx)?;
    let v3 = terrain.vertex(hex, v1_idx)?;

    // Add marker components to corner entities
    commands.entity(owner_entity).insert(QuadOwner);
    commands.entity(pos2_entity).insert(QuadPos2Emitter);
    commands.entity(pos3_entity).insert(QuadPos3Emitter);
    commands.entity(tail_entity).insert(QuadTail);

    // Build mesh in corner-local space
    let mesh = build_gap_mesh(&[v0, v1, v2, v3]);
    let mesh_entity = commands
        .spawn((
            Quad,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(gap_material.clone()),
            Transform::default(),
        ))
        .id();
    commands.entity(owner_entity).add_child(mesh_entity);
    Some(())
}

// ── Tri gap spawning ─────────────────────────────────────────────

fn spawn_tri(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    gap_material: &Handle<StandardMaterial>,
    terrain: &HGridLayout,
    corner_entities: &HashMap<(Hex, u8), Entity>,
    hex: Hex,
    vertex_index: u8,
) -> Option<()> {
    let dir = VertexDirection::ALL_DIRECTIONS[vertex_index as usize];
    let grid_vertex = hexx::GridVertex {
        origin: hex,
        direction: dir,
    };
    let coords = grid_vertex.coordinates();

    // Canonical ownership: this hex must be coords[0]
    (coords[0] == hex).then_some(())?;

    let v0_idx = dir.index();
    let idx1 = corner_index_for_vertex(coords[1], &grid_vertex)?;
    let idx2 = corner_index_for_vertex(coords[2], &grid_vertex)?;

    // All 3 corner entities must exist
    let &owner_entity = corner_entities.get(&(hex, v0_idx))?;
    let &pos1_entity = corner_entities.get(&(coords[1], idx1))?;
    let &pos2_entity = corner_entities.get(&(coords[2], idx2))?;

    // All 3 vertex positions
    let v0 = terrain.vertex(coords[0], v0_idx)?;
    let v1 = terrain.vertex(coords[1], idx1)?;
    let v2 = terrain.vertex(coords[2], idx2)?;

    // Add marker components to corner entities
    commands.entity(owner_entity).insert(TriOwner);
    commands.entity(pos1_entity).insert(TriPos1Emitter);
    commands.entity(pos2_entity).insert(TriPos2Emitter);

    // Build mesh in corner-local space
    let mesh = build_gap_mesh(&[v0, v1, v2]);
    let mesh_entity = commands
        .spawn((
            Tri,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(gap_material.clone()),
            Transform::default(),
        ))
        .id();
    commands.entity(owner_entity).add_child(mesh_entity);
    Some(())
}

// ── Helper functions ─────────────────────────────────────────────

/// Find which corner index on `hex` corresponds to the given vertex junction.
fn corner_index_for_vertex(hex: Hex, target: &hexx::GridVertex) -> Option<u8> {
    VertexDirection::ALL_DIRECTIONS.iter().find_map(|&dir| {
        let candidate = hexx::GridVertex {
            origin: hex,
            direction: dir,
        };
        candidate.equivalent(target).then_some(dir.index())
    })
}

/// Builds a gap mesh (3 or 4 world-space vertices) in the first vertex's local space.
fn build_gap_mesh(world_verts: &[Vec3]) -> Mesh {
    let origin = world_verts[0];
    let local: Vec<Vec3> = world_verts.iter().map(|&v| v - origin).collect();

    let normal = math::compute_normal(local[0], local[1], local[2]);
    let positions: Vec<[f32; 3]> = local.iter().map(|v| v.to_array()).collect();
    let normals = vec![normal.to_array(); positions.len()];

    let (uvs, indices): (Vec<[f32; 2]>, Vec<u16>) = if world_verts.len() == 4 {
        (
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![0, 1, 2, 0, 2, 3],
        )
    } else {
        (vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]], vec![0, 1, 2])
    };

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U16(indices))
}
