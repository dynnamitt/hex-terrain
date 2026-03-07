//! Centralized material definitions for height-based terrain.

use bevy::prelude::*;

/// Material handles for [`super::entities::InFov`] highlighting and terrain rendering.
#[derive(Resource)]
pub struct FovMaterials {
    /// Original hex face material.
    pub hex_original: Handle<StandardMaterial>,
    /// Highlight hex face material (emissive warm glow).
    pub hex_highlight: Handle<StandardMaterial>,
    /// Original gap (Quad/Tri) material.
    pub gap_original: Handle<StandardMaterial>,
    /// Highlight gap material (emissive cyan glow).
    pub gap_highlight: Handle<StandardMaterial>,
    /// Purple emissive material for the aimed-at hex face (screen center + within FoV).
    pub hex_in_aim: Handle<StandardMaterial>,
    /// Bright emissive edge-line material for quad edges.
    pub edge: Handle<StandardMaterial>,
}

impl FovMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            hex_original: materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.5, 0.1),
                ..default()
            }),
            hex_highlight: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.75, 0.15),
                emissive: LinearRgba::rgb(0.36, 0.2, 0.04),
                ..default()
            }),
            gap_original: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.04),
                emissive: LinearRgba::rgb(0.0255, 0.051, 0.085),
                cull_mode: None,
                ..default()
            }),
            gap_highlight: materials.add(StandardMaterial {
                base_color: Color::srgb(4.0, 4.0, 4.0),
                emissive: LinearRgba::rgb(0.01, 0.01, 0.01),
                cull_mode: None,
                ..default()
            }),
            hex_in_aim: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.1, 0.8),
                emissive: LinearRgba::rgb(0.92, 0.32, 2.56),
                ..default()
            }),
            edge: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.5, 1.0),
                emissive: LinearRgba::rgb(0.0, 20.0, 40.0),
                unlit: true,
                ..default()
            }),
        }
    }

    pub fn debug_material(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.2, 0.8),
            emissive: LinearRgba::rgb(4.0, 0.8, 3.2),
            unlit: true,
            ..default()
        })
    }
}
