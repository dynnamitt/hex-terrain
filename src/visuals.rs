//! Scene visuals: camera, bloom, tonemapping, and shared materials.
//!
//! Sets up the `Camera3d` with HDR + bloom post-processing and creates the
//! [`ActiveNeonMaterials`] resource consumed by `grid` and `edges` when spawning
//! geometry.

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::{Bloom, BloomCompositeMode};
use bevy::prelude::*;
use bevy::render::view::Hdr;

/// Sets up the camera, bloom, tonemapping, and shared neon materials.
pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_visuals);
    }
}

/// Shared material handles for the neon visual theme.
#[derive(Resource)]
pub struct ActiveNeonMaterials {
    /// Bright emissive cyan used for edge lines.
    pub edge_material: Handle<StandardMaterial>,
    /// Dark surface material applied to hex face meshes.
    pub hex_face_material: Handle<StandardMaterial>,
    /// Slightly warm dark material for gap-fill quads and triangles.
    pub gap_face_material: Handle<StandardMaterial>,
}

/// Spawns the camera entity and inserts [`ActiveNeonMaterials`].
pub fn setup_visuals(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    // Camera with bloom and tonemapping
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: 0.3,
            composite_mode: BloomCompositeMode::Additive,
            ..Bloom::NATURAL
        },
        Transform::from_xyz(0.0, 12.0, 0.0).looking_at(Vec3::new(5.0, 0.0, 5.0), Vec3::Y),
        crate::camera::TerrainCamera,
    ));

    // Clear color: near-black
    commands.insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.02)));

    // Neon materials
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
        // channel?
        emissive: LinearRgba::rgb(0.03, 0.06, 0.1),
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(ActiveNeonMaterials {
        edge_material,
        hex_face_material,
        gap_face_material,
    });
}
