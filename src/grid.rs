//! Hex grid generation: noise heights, per-hex radii, and vertex positions.
//!
//! Builds the [`HexGrid`] resource at startup using Perlin-based fractal noise
//! for terrain heights and per-hex radii. Each hex also gets a flat face mesh
//! spawned here; petal geometry is handled by [`crate::petals`].

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{Hex, HexLayout, PlaneMeshBuilder, VertexDirection, shapes};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::math;
use crate::petals::{HexEntities, HexSunDisc};
use crate::visuals::ActiveNeonMaterials;

/// Marker for height-indicator pole entities.
#[derive(Component, Reflect)]
pub struct HeightPole;

/// Per-plugin configuration for the hex grid generator.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct GridConfig {
    /// Number of hex rings around the origin (~1200 hexes at 20).
    pub grid_radius: u32,
    /// Distance in world-units between adjacent hex centers.
    pub point_spacing: f32,
    /// Maximum terrain elevation produced by the noise function.
    pub max_height: f32,
    /// Smallest visual hex radius (noise-derived per cell).
    pub min_hex_radius: f32,
    /// Largest visual hex radius (noise-derived per cell).
    pub max_hex_radius: f32,
    /// Pole cylinder radius as a fraction of the hex's visual radius.
    pub pole_radius_factor: f32,
    /// Distance at which poles reach full opacity.
    pub pole_fade_distance: f32,
    /// Minimum alpha when the camera is right on top of a pole.
    pub pole_min_alpha: f32,
    /// Gap between pole top and hex face.
    pub pole_gap: f32,
    /// Seed for the height noise generator.
    pub height_noise_seed: u32,
    /// Seed for the per-hex radius noise generator.
    pub radius_noise_seed: u32,
    /// Number of octaves for height noise.
    pub height_noise_octaves: usize,
    /// Number of octaves for radius noise.
    pub radius_noise_octaves: usize,
    /// Spatial scale divisor for height noise sampling.
    pub height_noise_scale: f64,
    /// Spatial scale divisor for radius noise sampling.
    pub radius_noise_scale: f64,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            grid_radius: 20,
            point_spacing: 4.0,
            max_height: 10.0,
            min_hex_radius: 0.2,
            max_hex_radius: 2.6,
            pole_radius_factor: 0.06,
            pole_fade_distance: 40.0,
            pole_min_alpha: 0.05,
            pole_gap: 0.05,
            height_noise_seed: 42,
            radius_noise_seed: 137,
            height_noise_octaves: 4,
            radius_noise_octaves: 3,
            height_noise_scale: 50.0,
            radius_noise_scale: 30.0,
        }
    }
}

/// Registers the [`generate_grid`] startup system.
pub struct GridPlugin(pub GridConfig);

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightPole>()
            .register_type::<GridConfig>()
            .insert_resource(self.0.clone())
            .add_systems(Startup, generate_grid.after(crate::visuals::setup_visuals))
            .add_systems(Update, fade_nearby_poles);
    }
}

/// Central resource holding the hex layout, per-cell noise data, and vertex positions.
#[derive(Resource)]
pub struct HexGrid {
    /// Hex-to-world coordinate mapping (spacing, orientation).
    pub layout: HexLayout,
    /// Noise-derived terrain height for each hex cell.
    pub heights: HashMap<Hex, f32>,
    /// Noise-derived visual radius for each hex cell.
    pub radii: HashMap<Hex, f32>,
    /// World-space position of each hex vertex, keyed by `(hex, vertex_index 0..5)`.
    pub vertex_positions: HashMap<(Hex, u8), Vec3>,
}

/// Builds the [`HexGrid`] resource and spawns a flat face mesh for every hex cell.
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
        hex_entity_map.insert(hex, entity);

        // Height indicator pole: from y=0 up to just below the hex face
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
            commands.spawn((
                HeightPole,
                Name::new(format!("Pole({},{})", hex.x, hex.y)),
                Mesh3d(pole_mesh_handle.clone()),
                MeshMaterial3d(pole_mat),
                Transform::from_xyz(center_2d.x, pg.y_center, center_2d.y).with_scale(Vec3::new(
                    pole_radius / 0.5,
                    pg.height,
                    pole_radius / 0.5,
                )),
            ));
        }
    }

    commands.insert_resource(HexGrid {
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
fn fade_nearby_poles(
    camera_q: Query<&Transform, With<crate::camera::TerrainCamera>>,
    pole_q: Query<(&Transform, &MeshMaterial3d<StandardMaterial>), With<HeightPole>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<GridConfig>,
) {
    let Ok(cam_tf) = camera_q.single() else {
        return;
    };
    let cam_xz = Vec2::new(cam_tf.translation.x, cam_tf.translation.z);

    for (pole_tf, mat_handle) in &pole_q {
        let pole_xz = Vec2::new(pole_tf.translation.x, pole_tf.translation.z);
        let dist = cam_xz.distance(pole_xz);
        let brightness =
            math::pole_fade_brightness(dist, cfg.pole_fade_distance, cfg.pole_min_alpha);

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgb(0.0, brightness, 0.2 * brightness);
            mat.emissive = LinearRgba::rgb(0.0, 30.0 * brightness, 6.0 * brightness);
        }
    }
}
