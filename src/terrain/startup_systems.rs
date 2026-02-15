use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{Hex, HexLayout, PlaneMeshBuilder, shapes};

use super::TerrainConfig;
use super::entities::{HexEntities, HexGrid, HexSunDisc, NeonMaterials, Stem};
use super::terrain_hex_layout::TerrainHexLayout;
use crate::math;

// ── Startup ─────────────────────────────────────────────────────────

/// Spawns the [`HexGrid`] entity, neon materials, and a flat face mesh for every hex cell.
pub fn generate_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<TerrainConfig>,
) {
    // Create neon materials
    let edge_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.5, 1.0),
        emissive: LinearRgba::rgb(0.0, 20.0, 40.0),
        unlit: true,
        ..default()
    });
    let hex_face_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.02, 0.03, 0.05),
        emissive: LinearRgba::rgb(0.02, 0.05, 0.08),
        ..default()
    });
    let gap_face_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.03, 0.05),
        emissive: LinearRgba::rgb(0.03, 0.06, 0.1),
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(NeonMaterials {
        edge_material,
        gap_face_material,
    });

    let g = &cfg.grid;
    let f = &cfg.flower;
    let terrain = TerrainHexLayout::from_settings(g);

    // Spawn hex face meshes
    let unit_layout = HexLayout {
        scale: Vec2::splat(1.0),
        ..default()
    };
    let hex_mesh_info = PlaneMeshBuilder::new(&unit_layout).build();
    let hex_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, hex_mesh_info.vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, hex_mesh_info.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, hex_mesh_info.uvs)
    .with_inserted_indices(Indices::U16(hex_mesh_info.indices));
    let hex_mesh_handle = meshes.add(hex_mesh);

    let stem_mesh_handle = meshes.add(Cylinder::new(0.5, 1.0));

    let grid_entity = commands
        .spawn((
            Name::new("HexGrid"),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    let mut hex_entity_map: HashMap<Hex, Entity> = HashMap::new();

    for hex in shapes::hexagon(Hex::ZERO, g.radius) {
        let center_2d = terrain.hex_to_world_pos(hex);
        let height = terrain.height(&hex).unwrap();
        let radius = terrain.radius(&hex).unwrap();

        let entity = commands
            .spawn((
                HexSunDisc { hex },
                super::entities::FlowerState::Naked,
                Name::new(format!("HexSunDisc({},{})", hex.x, hex.y)),
                Mesh3d(hex_mesh_handle.clone()),
                MeshMaterial3d(hex_face_material.clone()),
                Transform::from_xyz(center_2d.x, height, center_2d.y)
                    .with_scale(Vec3::new(radius, 1.0, radius)),
            ))
            .id();
        commands.entity(grid_entity).add_child(entity);
        hex_entity_map.insert(hex, entity);

        // Height indicator stem
        if let Some(sg) = math::stem_geometry(radius, height, f.stem_radius_factor, f.stem_gap) {
            let stem_radius = sg.radius;
            let stem_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.2),
                emissive: LinearRgba::rgb(0.0, 30.0, 6.0),
                unlit: true,
                ..default()
            });
            let stem_entity = commands
                .spawn((
                    Stem,
                    Name::new(format!("Stem({},{})", hex.x, hex.y)),
                    Mesh3d(stem_mesh_handle.clone()),
                    MeshMaterial3d(stem_mat),
                    Transform::from_xyz(0.0, sg.y_center - height, 0.0).with_scale(Vec3::new(
                        stem_radius / 0.5 / radius,
                        sg.height,
                        stem_radius / 0.5 / radius,
                    )),
                ))
                .id();
            commands.entity(entity).add_child(stem_entity);
        }
    }

    commands.entity(grid_entity).insert(HexGrid { terrain });
    commands.insert_resource(HexEntities {
        map: hex_entity_map,
    });
}
