//! Startup systems for height-based terrain.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{Hex, HexLayout, PlaneMeshBuilder, shapes};

use flora::{FloraCfg, FloraMaterials};
use mesh_gradient::BlendCfg;

use super::HTerrainConfig;
use super::entities::{Corner, HCell, HGrid, HexFace, Quad, Tri};
use super::gaps;
use super::materials::TerrainMaterials;
use super::mineral::{HIGHLIGHT_EMISSIVE, HIGHLIGHT_MIX, Mineral};
use crate::DebugFlag;
use hex_grid::{edge_cuboid_transform, gap_filler};

/// Spawns the [`HGrid`] entity with [`HCell`] children, [`Corner`] grandchildren,
/// and Quad/Tri gap geometry with distributed emitter markers.
pub fn generate_h_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cfg: Res<HTerrainConfig>,
    debug: Res<DebugFlag>,
) {
    let g = &cfg.grid;
    let terrain = g.build_layout();

    let edge_thickness = 0.02;
    let fov = TerrainMaterials::new(&mut materials, &mut meshes, &mut images);
    let flora_cfg = FloraCfg::default();
    let flora_mat = FloraMaterials::new(&mut materials, &mut meshes, &flora_cfg);
    let debug_assets = debug.0.then(|| {
        let sphere_mesh = meshes.add(Sphere::new(0.08));
        let material = TerrainMaterials::debug_material(&mut materials);
        (sphere_mesh, material)
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

    let grid_entity = commands
        .spawn((
            Name::new("HGrid"),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    // Pre-create per-mineral normal + highlight material handles
    let mineral_handles: [Handle<StandardMaterial>; Mineral::COUNT] =
        Mineral::ALL.map(|m| materials.add(m.material()));
    let highlight_handles: [Handle<StandardMaterial>; Mineral::COUNT] =
        Mineral::ALL.map(|m| materials.add(m.highlight_material()));

    // ── Pass 1: Spawn HCells + Corners, build lookup maps ────────
    let mut corner_entities: HashMap<(Hex, u8), Entity> = HashMap::new();
    let mut hex_entities: HashMap<Hex, Entity> = HashMap::new();
    let mut hex_minerals: HashMap<Hex, Mineral> = HashMap::new();

    for hex in shapes::hexagon(Hex::ZERO, g.radius) {
        let center = terrain.hex_to_world_pos(hex);
        let height = terrain.height(&hex).unwrap();
        let radius = terrain.radius(&hex).unwrap();
        let mineral = cfg.biome.pick(hex, g.mineral_seed);
        hex_minerals.insert(hex, mineral);

        let cell_entity = commands
            .spawn((
                HCell { hex },
                mineral,
                Name::new(format!("HCell({},{})", hex.x, hex.y)),
                Transform::from_xyz(center.x, height, center.y),
                Visibility::default(),
            ))
            .with_child((
                HexFace,
                mineral,
                Mesh3d(hex_mesh.clone()),
                MeshMaterial3d(mineral_handles[mineral.idx()].clone()),
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
                let (midpoint, length, rotation) = edge_cuboid_transform(Vec3::ZERO, edge_vec);
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
    {
        let blend_cfg = BlendCfg::default();
        let dbg_normal = debug.0.then(|| {
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0),
                unlit: true,
                ..default()
            });
            let mesh = meshes.add(Cuboid::new(
                gaps::DBG_NORMAL_THICKNESS,
                gaps::DBG_NORMAL_LEN,
                gaps::DBG_NORMAL_THICKNESS,
            ));
            (mat, mesh)
        });
        let edge_mesh = meshes.add(Cuboid::new(1.0, gaps::EDGE_THICKNESS, gaps::EDGE_THICKNESS));
        let mut ctx = gaps::GapSpawnCtx {
            materials: &mut materials,
            meshes: &mut meshes,
            images: &mut images,
            mineral_handles: &mineral_handles,
            highlight_handles: &highlight_handles,
            edge_material: &fov.edge,
            edge_mesh: &edge_mesh,
            blend_cfg: &blend_cfg,
            highlight_mix: HIGHLIGHT_MIX,
            highlight_emissive: HIGHLIGHT_EMISSIVE,
            flat_normals: g.flat_gap_normals,
            terrain: &terrain,
            corner_entities: &corner_entities,
            hex_entities: &hex_entities,
            hex_minerals: &hex_minerals,
            quad_cache: HashMap::new(),
            tri_cache: HashMap::new(),
            dbg_normal,
        };

        for hex in shapes::hexagon(Hex::ZERO, g.radius) {
            for edge_index in [0u8, 2, 4] {
                gaps::spawn_quad(&mut commands, &mut ctx, hex, edge_index);
            }
            for vertex_index in [0u8, 1] {
                gaps::spawn_tri(&mut commands, &mut ctx, hex, vertex_index);
            }
        }
    }

    // ── Pass 3: Flora clusters on eligible hexes ─────────────────
    super::flora_spawn::spawn_flora(
        &mut commands,
        &flora_mat,
        &flora_cfg,
        &hex_minerals,
        &hex_entities,
        g.flora_seed,
    );

    commands.entity(grid_entity).insert(HGrid {
        terrain,
        hex_entities,
    });
    commands.insert_resource(fov);
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
    let (expected_quads, expected_tris) = gap_filler(&hexes);
    let actual_quads = quads.iter().count();
    let actual_tris = tris.iter().count();

    assert_eq!(
        (actual_quads, actual_tris),
        (expected_quads, expected_tris),
        "Gap entity mismatch: got ({actual_quads}, {actual_tris}), expected ({expected_quads}, {expected_tris})"
    );
    info!("Gap counts verified: {actual_quads} quads, {actual_tris} tris");
}
