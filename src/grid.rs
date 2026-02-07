use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{shapes, Hex, HexLayout, PlaneMeshBuilder, VertexDirection};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::visuals::NeonMaterials;
use crate::{AppConfig, HeightMode};

pub const GRID_RADIUS: u32 = 20;
pub const POINT_SPACING: f32 = 4.0;
pub const HEX_RENDER_SIZE: f32 = 1.0;
pub const MAX_HEIGHT: f32 = 10.0;
pub const CAMERA_HEIGHT_OFFSET: f32 = 2.0;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_grid.after(crate::visuals::setup_visuals));
    }
}

#[derive(Component)]
pub struct Point {
    pub hex: Hex,
    pub height: f32,
}

#[derive(Resource)]
pub struct HexGrid {
    pub layout: HexLayout,
    pub render_layout: HexLayout,
    pub heights: HashMap<Hex, f32>,
    pub vertex_positions: HashMap<(Hex, u8), Vec3>,
}

pub fn generate_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    neon: Res<NeonMaterials>,
    config: Res<AppConfig>,
) {
    let layout = HexLayout {
        scale: Vec2::splat(POINT_SPACING),
        ..default()
    };
    let render_layout = HexLayout {
        scale: Vec2::splat(HEX_RENDER_SIZE),
        ..default()
    };

    // Generate noise heights
    let fbm: Fbm<Perlin> = Fbm::new(42).set_octaves(4);
    let mut heights: HashMap<Hex, f32> = HashMap::new();

    for hex in shapes::hexagon(Hex::ZERO, GRID_RADIUS) {
        let pos = layout.hex_to_world_pos(hex);
        let noise_val = fbm.get([pos.x as f64 / 50.0, pos.y as f64 / 50.0]);
        // Map noise from [-1, 1] to [0, MAX_HEIGHT]
        let h = ((noise_val as f32 + 1.0) / 2.0) * MAX_HEIGHT;
        heights.insert(hex, h);
    }

    // Compute vertex positions
    let mut vertex_positions: HashMap<(Hex, u8), Vec3> = HashMap::new();
    let corner_offsets = render_layout.center_aligned_hex_corners();

    for hex in shapes::hexagon(Hex::ZERO, GRID_RADIUS) {
        let center_2d = layout.hex_to_world_pos(hex);
        let center_height = heights[&hex];

        for (i, dir) in VertexDirection::ALL_DIRECTIONS.iter().enumerate() {
            let offset_2d = corner_offsets[i];
            let world_x = center_2d.x + offset_2d.x;
            let world_z = center_2d.y + offset_2d.y;

            let vertex_height = match config.height_mode {
                HeightMode::Blocky => center_height,
                HeightMode::Smooth => {
                    // Find the hexes that share this vertex
                    let grid_vertex = hexx::GridVertex {
                        origin: hex,
                        direction: *dir,
                    };
                    let coords = grid_vertex.coordinates();
                    let mut sum = 0.0;
                    let mut count = 0;
                    for coord in &coords {
                        if let Some(&h) = heights.get(coord) {
                            sum += h;
                            count += 1;
                        }
                    }
                    if count > 0 { sum / count as f32 } else { center_height }
                }
            };

            vertex_positions.insert((hex, i as u8), Vec3::new(world_x, vertex_height, world_z));
        }
    }

    // Spawn hex face meshes using PlaneMeshBuilder
    let hex_mesh_info = PlaneMeshBuilder::new(&render_layout).build();
    let hex_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, hex_mesh_info.vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, hex_mesh_info.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, hex_mesh_info.uvs)
    .with_inserted_indices(Indices::U16(hex_mesh_info.indices));
    let hex_mesh_handle = meshes.add(hex_mesh);

    for hex in shapes::hexagon(Hex::ZERO, GRID_RADIUS) {
        let center_2d = layout.hex_to_world_pos(hex);
        let center_height = heights[&hex];

        // For smooth mode, use average of the 6 vertex heights as face center
        let face_height = if config.height_mode == HeightMode::Smooth {
            let mut sum = 0.0;
            for i in 0..6u8 {
                sum += vertex_positions[&(hex, i)].y;
            }
            sum / 6.0
        } else {
            center_height
        };

        commands.spawn((
            Point {
                hex,
                height: center_height,
            },
            Mesh3d(hex_mesh_handle.clone()),
            MeshMaterial3d(neon.hex_face_material.clone()),
            Transform::from_xyz(center_2d.x, face_height, center_2d.y),
        ));
    }

    commands.insert_resource(HexGrid {
        layout,
        render_layout,
        heights,
        vertex_positions,
    });
}

