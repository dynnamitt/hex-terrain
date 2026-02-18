//! First-person drone controller.
//!
//! WASD + mouse look + Q/E/scroll altitude. Writes to [`PlayerPos`](crate::PlayerPos)
//! for terrain to consume. Spawns the Camera3d entity with bloom.

mod entities;
mod systems;

pub use entities::Player;

use bevy::prelude::*;

use crate::GameState;

/// Per-plugin configuration for the drone controller.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct DroneConfig {
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
    /// Altitude change per scroll line.
    pub scroll_sensitivity: f32,
    /// Bloom post-processing intensity.
    pub bloom_intensity: f32,
    /// Height lerp factor for smooth camera Y transitions.
    pub height_lerp: f32,
    /// Initial altitude offset above terrain when spawning.
    pub spawn_altitude: f32,
}

impl Default for DroneConfig {
    fn default() -> Self {
        Self {
            move_speed: 15.0,
            mouse_sensitivity_x: 0.003,
            mouse_sensitivity_y: 0.002,
            edge_margin: 100.0,
            pitch_margin: 0.05,
            scroll_sensitivity: 3.0,
            bloom_intensity: 0.3,
            height_lerp: 0.1,
            spawn_altitude: 12.0,
        }
    }
}

/// First-person drone controller with WASD, mouse look, and altitude control.
pub struct DronePlugin(pub DroneConfig);

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Player>()
            .register_type::<DroneConfig>()
            .insert_resource(self.0.clone())
            .init_resource::<entities::CursorRecentered>()
            .add_systems(Startup, (systems::spawn_drone, systems::hide_cursor))
            .add_systems(
                Update,
                systems::recenter_cursor.run_if(not(in_state(GameState::Inspecting))),
            )
            .add_systems(
                Update,
                systems::fly
                    .after(systems::recenter_cursor)
                    .run_if(in_state(GameState::Running)),
            );
    }
}
