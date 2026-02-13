//! First-person camera controller and hex-cell tracking.
//!
//! WASD + mouse look with terrain-height interpolation. [`CameraCell`] reports
//! which hex the camera currently occupies; downstream systems in `petals` use
//! its change flag to spawn geometry and restyle visited cells.

mod entities;
mod systems;

pub use entities::{CameraCell, TerrainCamera};
pub use systems::{interpolate_height, track_camera_cell};

use bevy::prelude::*;

use crate::InspectorActive;
use crate::intro::IntroSequence;

/// Per-plugin configuration for the camera controller.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct CameraConfig {
    /// WASD movement speed in world-units per second.
    pub move_speed: f32,
    /// Horizontal mouse sensitivity (radians per pixel).
    pub mouse_sensitivity_x: f32,
    /// Vertical mouse sensitivity (radians per pixel).
    pub mouse_sensitivity_y: f32,
    /// Pixel margin from window edge that triggers cursor recentering.
    pub edge_margin: f32,
    /// Margin from vertical to prevent camera flip (radians).
    pub pitch_margin: f32,
    /// Lerp factor for smooth height transitions per frame.
    pub height_lerp: f32,
    /// Vertical offset of the camera above the terrain surface.
    pub height_offset: f32,
    /// Altitude change per scroll line.
    pub scroll_sensitivity: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            move_speed: 15.0,
            mouse_sensitivity_x: 0.003,
            mouse_sensitivity_y: 0.002,
            edge_margin: 100.0,
            pitch_margin: 0.05,
            height_lerp: 0.1,
            height_offset: 16.0,
            scroll_sensitivity: 3.0,
        }
    }
}

/// First-person camera controller with WASD movement, mouse look, and terrain following.
pub struct CameraPlugin(pub CameraConfig);

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        let altitude = entities::CameraAltitude(self.0.height_offset);
        app.register_type::<TerrainCamera>()
            .register_type::<CameraConfig>()
            .insert_resource(self.0.clone())
            .insert_resource(altitude)
            .init_resource::<CameraCell>()
            .init_resource::<entities::CursorRecentered>()
            .add_systems(Startup, systems::hide_cursor)
            .add_systems(
                Update,
                systems::recenter_cursor.run_if(|active: Res<InspectorActive>| !active.0),
            )
            .add_systems(
                Update,
                (systems::move_camera, track_camera_cell)
                    .chain()
                    .after(systems::recenter_cursor)
                    .run_if(|intro: Res<IntroSequence>| intro.done)
                    .run_if(|active: Res<InspectorActive>| !active.0),
            );
    }
}
