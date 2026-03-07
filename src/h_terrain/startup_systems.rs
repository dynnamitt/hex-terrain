//! Startup systems for height-based terrain.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, Hex, HexLayout, PlaneMeshBuilder, VertexDirection, shapes};

use super::HTerrainConfig;
use super::entities::{
    Corner, FovMaterials, HCell, HGrid, HexFace, Quad, QuadEdge, QuadOwner, QuadPos2Emitter,
    QuadPos3Emitter, QuadTail, Tri, TriOwner, TriPos1Emitter, TriPos2Emitter,
};
use super::h_grid_layout::HGridLayout;
use super::math;
use crate::DebugFlag;

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
        base_color: Color::srgb(0.102, 0.0255, 0.0425),
        emissive: LinearRgba::rgb(0.0255, 0.051, 0.085),
        cull_mode: None,
        ..default()
    });

    let edge_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.5, 1.0),
        emissive: LinearRgba::rgb(0.0, 20.0, 40.0),
        unlit: true,
        ..default()
    });

    // Unit hex face mesh (scaled per-hex by radius)
    let unit_layout = HexLayout {
        scale: Vec2::splat(1.0),
        ..default()
    };
    let hex_mesh_info = PlaneMeshBuilder::new(&unit_layout).build();
    let hex_mesh = meshes.add(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, hex_mesh_info.vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, hex_mesh_info.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, hex_mesh_info.uvs)
        .with_inserted_indices(Indices::U16(hex_mesh_info.indices)),
    );
    let hex_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.5, 0.1),
        ..default()
    });

    let hex_highlight = materials.add(StandardMaterial {
        base_color: Color::srgb(0.042, 0.126, 0.168),
        emissive: LinearRgba::rgb(0.168, 0.672, 1.344),
        ..default()
    });
    let gap_highlight = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.12, 0.16),
        emissive: LinearRgba::rgb(0.16, 0.64, 1.28),
        cull_mode: None,
        ..default()
    });
    let hex_in_sight = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.1, 0.8),
        emissive: LinearRgba::rgb(2.4, 0.4, 3.2),
        ..default()
    });
    commands.insert_resource(FovMaterials {
        hex_original: hex_material.clone(),
        hex_highlight,
        gap_original: gap_material.clone(),
        gap_highlight,
        hex_in_sight,
    });

    let grid_entity = commands
        .spawn((
            Name::new("HGrid"),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    // ── Pass 1: Spawn HCells + Corners, build lookup maps ────────
    let mut corner_entities: HashMap<(Hex, u8), Entity> = HashMap::new();
    let mut hex_entities: HashMap<Hex, Entity> = HashMap::new();

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
            .with_child((
                HexFace,
                Mesh3d(hex_mesh.clone()),
                MeshMaterial3d(hex_material.clone()),
                Transform::from_scale(Vec3::new(radius, 1.0, radius)),
            ))
            .id();
        hex_entities.insert(hex, cell_entity);

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
                let (midpoint, length, rotation) =
                    math::edge_cuboid_transform(Vec3::ZERO, edge_vec);
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
                &edge_material,
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

    commands.entity(grid_entity).insert(HGrid {
        terrain,
        hex_entities,
    });
}

// ── Quad gap spawning ────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn spawn_quad(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    gap_material: &Handle<StandardMaterial>,
    edge_material: &Handle<StandardMaterial>,
    terrain: &HGridLayout,
    corner_entities: &HashMap<(Hex, u8), Entity>,
    hex: Hex,
    edge_index: u8,
) -> Option<()> {
    let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
    let neighbor = hex.neighbor(dir);

    let (v0_idx, v1_idx, n0_idx, n1_idx) = math::quad_corner_indices(edge_index);

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

    // Build mesh in corner-local space
    let mesh = math::build_gap_mesh(&[v0, v1, v2, v3]);
    let mesh_entity = commands
        .spawn((
            Quad,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(gap_material.clone()),
            Transform::default(),
        ))
        .id();
    commands.entity(owner_entity).add_child(mesh_entity);

    // Add marker components to corner entities
    commands.entity(owner_entity).insert(QuadOwner);
    commands
        .entity(pos2_entity)
        .insert(QuadPos2Emitter(mesh_entity));
    commands
        .entity(pos3_entity)
        .insert(QuadPos3Emitter(mesh_entity));
    commands.entity(tail_entity).insert(QuadTail);

    // Spawn edge lines as children of the Quad mesh entity
    let edge_thickness = 0.03;
    let origin = v0;
    let edges = [(v0, v3), (v1, v2), (v0, v1), (v3, v2)];
    for (from, to) in edges {
        let local_from = from - origin;
        let local_to = to - origin;
        let (midpoint, length, rotation) = math::edge_cuboid_transform(local_from, local_to);
        let edge_entity = commands
            .spawn((
                QuadEdge,
                Mesh3d(meshes.add(Cuboid::new(length, edge_thickness, edge_thickness))),
                MeshMaterial3d(edge_material.clone()),
                Transform::from_translation(midpoint).with_rotation(rotation),
            ))
            .id();
        commands.entity(mesh_entity).add_child(edge_entity);
    }

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

    // Build mesh in corner-local space
    let mesh = math::build_gap_mesh(&[v0, v1, v2]);
    let mesh_entity = commands
        .spawn((
            Tri,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(gap_material.clone()),
            Transform::default(),
        ))
        .id();
    commands.entity(owner_entity).add_child(mesh_entity);

    // Add marker components to corner entities
    commands.entity(owner_entity).insert(TriOwner);
    commands
        .entity(pos1_entity)
        .insert(TriPos1Emitter(mesh_entity));
    commands
        .entity(pos2_entity)
        .insert(TriPos2Emitter(mesh_entity));
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

/// Seeds [`GroundLevel`](crate::GroundLevel) from terrain height at the origin.
///
/// Runs at startup (after grid generation) so that the ground level is
/// correct before the drone spawns.
pub fn seed_ground_level(grid: Single<&HGrid>, mut ground: ResMut<crate::GroundLevel>) {
    ground.0 = Some(grid.terrain.interpolate_height(Vec2::ZERO));
}

/// Debug-only startup check: asserts spawned Quad/Tri counts match `gap_filler` expectations.
pub fn verify_gap_counts(
    _grid: Single<&HGrid>,
    quads: Query<(), With<Quad>>,
    tris: Query<(), With<Tri>>,
    cfg: Res<HTerrainConfig>,
) {
    let hexes: Vec<Hex> = shapes::hexagon(Hex::ZERO, cfg.grid.radius).collect();
    let (expected_quads, expected_tris) = math::gap_filler(&hexes);
    let actual_quads = quads.iter().count();
    let actual_tris = tris.iter().count();

    assert_eq!(
        (actual_quads, actual_tris),
        (expected_quads, expected_tris),
        "Gap entity mismatch: got ({actual_quads}, {actual_tris}), expected ({expected_quads}, {expected_tris})"
    );
    info!("Gap counts verified: {actual_quads} quads, {actual_tris} tris");
}
