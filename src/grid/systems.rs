use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{Hex, HexLayout, PlaneMeshBuilder, VertexDirection, shapes};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

#[cfg(debug_assertions)]
use bevy_egui::egui;

use super::GridConfig;
use super::entities::HexGrid;
use crate::math;
use crate::petals::{HeightPole, HexEntities, HexSunDisc};
use crate::visuals::ActiveNeonMaterials;

/// Spawns the [`HexGrid`] entity and a flat face mesh for every hex cell.
#[allow(clippy::too_many_arguments)]
pub fn generate_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    neon: Res<ActiveNeonMaterials>,
    cfg: Res<GridConfig>,
) {
    let layout = HexLayout {
        scale: Vec2::splat(cfg.point_spacing),
        ..default()
    };
    let unit_layout = HexLayout {
        scale: Vec2::splat(1.0),
        ..default()
    };

    // Generate noise for heights and sizes
    let height_fbm: Fbm<Perlin> =
        Fbm::new(cfg.height_noise_seed).set_octaves(cfg.height_noise_octaves);
    let radius_fbm: Fbm<Perlin> =
        Fbm::new(cfg.radius_noise_seed).set_octaves(cfg.radius_noise_octaves);
    let mut heights: HashMap<Hex, f32> = HashMap::new();
    let mut radii: HashMap<Hex, f32> = HashMap::new();

    for hex in shapes::hexagon(Hex::ZERO, cfg.grid_radius) {
        let pos = layout.hex_to_world_pos(hex);

        let noise_val = height_fbm.get([
            pos.x as f64 / cfg.height_noise_scale,
            pos.y as f64 / cfg.height_noise_scale,
        ]);
        let h = math::map_noise_to_range(noise_val, 0.0, cfg.max_height);
        heights.insert(hex, h);

        let radius_noise = radius_fbm.get([
            pos.x as f64 / cfg.radius_noise_scale,
            pos.y as f64 / cfg.radius_noise_scale,
        ]);
        let r = math::map_noise_to_range(radius_noise, cfg.min_hex_radius, cfg.max_hex_radius);
        radii.insert(hex, r);
    }

    // Compute vertex positions
    let mut vertex_positions: HashMap<(Hex, u8), Vec3> = HashMap::new();
    let unit_offsets = unit_layout.center_aligned_hex_corners();

    for hex in shapes::hexagon(Hex::ZERO, cfg.grid_radius) {
        let center_2d = layout.hex_to_world_pos(hex);
        let center_height = heights[&hex];
        let radius = radii[&hex];

        for (i, _dir) in VertexDirection::ALL_DIRECTIONS.iter().enumerate() {
            let offset_2d = unit_offsets[i] * radius;
            let world_x = center_2d.x + offset_2d.x;
            let world_z = center_2d.y + offset_2d.y;

            let vertex_height = center_height;

            vertex_positions.insert((hex, i as u8), Vec3::new(world_x, vertex_height, world_z));
        }
    }

    // Spawn hex face meshes using PlaneMeshBuilder with unit-radius layout
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

    // Unit cylinder (radius 0.5, height 1) — scaled per hex
    let pole_mesh_handle = meshes.add(Cylinder::new(0.5, 1.0));

    let grid_entity = commands
        .spawn((
            Name::new("HexGrid"),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    let mut hex_entity_map: HashMap<Hex, Entity> = HashMap::new();

    for hex in shapes::hexagon(Hex::ZERO, cfg.grid_radius) {
        let center_2d = layout.hex_to_world_pos(hex);
        let center_height = heights[&hex];
        let radius = radii[&hex];

        // For smooth mode, use average of the 6 vertex heights as face center
        let face_height = center_height;

        let entity = commands
            .spawn((
                HexSunDisc { hex },
                Name::new(format!("HexSunDisc({},{})", hex.x, hex.y)),
                Mesh3d(hex_mesh_handle.clone()),
                MeshMaterial3d(neon.hex_face_material.clone()),
                Transform::from_xyz(center_2d.x, face_height, center_2d.y)
                    .with_scale(Vec3::new(radius, 1.0, radius)),
            ))
            .id();
        commands.entity(grid_entity).add_child(entity);
        hex_entity_map.insert(hex, entity);

        // Height indicator pole: child of its HexSunDisc
        if let Some(pg) =
            math::pole_geometry(radius, face_height, cfg.pole_radius_factor, cfg.pole_gap)
        {
            let pole_radius = pg.radius;
            // Each pole gets its own material so alpha can vary per-pole
            let pole_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.2),
                emissive: LinearRgba::rgb(0.0, 30.0, 6.0),
                unlit: true,
                ..default()
            });
            // Local transform relative to parent HexSunDisc
            // Parent is at (center_2d.x, face_height, center_2d.y) with scale (radius, 1.0, radius)
            let pole_entity = commands
                .spawn((
                    HeightPole,
                    Name::new(format!("Pole({},{})", hex.x, hex.y)),
                    Mesh3d(pole_mesh_handle.clone()),
                    MeshMaterial3d(pole_mat),
                    Transform::from_xyz(0.0, pg.y_center - face_height, 0.0).with_scale(Vec3::new(
                        pole_radius / 0.5 / radius,
                        pg.height,
                        pole_radius / 0.5 / radius,
                    )),
                ))
                .id();
            commands.entity(entity).add_child(pole_entity);
        }
    }

    commands.entity(grid_entity).insert(HexGrid {
        layout,
        heights,
        radii,
        vertex_positions,
    });
    commands.insert_resource(HexEntities {
        map: hex_entity_map,
    });
}

/// Adjusts pole material alpha based on horizontal distance to the camera.
pub fn fade_nearby_poles(
    camera_q: Query<&Transform, With<crate::camera::TerrainCamera>>,
    pole_q: Query<(&GlobalTransform, &MeshMaterial3d<StandardMaterial>), With<HeightPole>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<GridConfig>,
) {
    let Ok(cam_tf) = camera_q.single() else {
        return;
    };
    let cam_xz = Vec2::new(cam_tf.translation.x, cam_tf.translation.z);

    for (pole_tf, mat_handle) in &pole_q {
        let pos = pole_tf.translation();
        let pole_xz = Vec2::new(pos.x, pos.z);
        let dist = cam_xz.distance(pole_xz);
        let brightness =
            math::pole_fade_brightness(dist, cfg.pole_fade_distance, cfg.pole_min_alpha);

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgb(0.0, brightness, 0.2 * brightness);
            mat.emissive = LinearRgba::rgb(0.0, 30.0 * brightness, 6.0 * brightness);
        }
    }
}

/// Draws the [`Name`] of each [`HexSunDisc`] as a screen-projected egui label.
#[cfg(debug_assertions)]
pub fn draw_hex_labels(
    mut egui_ctx: Query<&mut bevy_egui::EguiContext>,
    camera_q: Query<(&Camera, &GlobalTransform), With<crate::camera::TerrainCamera>>,
    hexes: Query<(&GlobalTransform, &Name), With<HexSunDisc>>,
    mut ready: Local<bool>,
) {
    // Egui fonts aren't available until after the first Context::run() in the render pass.
    if !*ready {
        *ready = true;
        return;
    }
    let Ok((camera, cam_gt)) = camera_q.single() else {
        return;
    };
    let Ok(mut ctx) = egui_ctx.single_mut() else {
        return;
    };
    let cam_pos = cam_gt.translation();

    let painter = ctx.get_mut().layer_painter(egui::LayerId::background());

    for (hex_gt, name) in &hexes {
        let world_pos = hex_gt.translation();
        if cam_pos.distance(world_pos) > 30.0 {
            continue;
        }
        if let Ok(viewport) = camera.world_to_viewport(cam_gt, world_pos) {
            painter.text(
                egui::pos2(viewport.x, viewport.y),
                egui::Align2::CENTER_CENTER,
                name.as_str(),
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
        }
    }
}
