//! First-person drone controller.
//!
//! WASD + mouse look + Q/E/scroll altitude. Writes to [`PlayerPos`](crate::PlayerPos)
//! for terrain to consume. Spawns the Camera3d entity with bloom.

mod entities;
pub(crate) mod systems;

pub use entities::Player;

use bevy::ecs::schedule::InternedSystemSet;
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
    /// Minimum offset above terrain (spawn height + floor for Q/scroll).
    pub lowest_offset: f32,
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
            lowest_offset: 2.0,
        }
    }
}

/// First-person drone controller with WASD, mouse look, and altitude control.
pub struct DronePlugin {
    /// Per-plugin configuration.
    pub config: DroneConfig,
    /// Optional Startup set that `spawn_drone` must run after (terrain seed).
    pub after_terrain_seed: Option<InternedSystemSet>,
}

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Player>()
            .register_type::<DroneConfig>()
            .insert_resource(self.config.clone())
            .init_resource::<entities::CursorRecentered>();

        if let Some(set) = self.after_terrain_seed {
            app.add_systems(Startup, systems::spawn_drone.after(set));
        } else {
            app.add_systems(Startup, systems::spawn_drone);
        }

        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Startup, systems::hide_cursor)
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

        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, systems::fly.run_if(in_state(GameState::Running)))
            .add_systems(
                Update,
                systems::lock_cursor_on_click.run_if(not(in_state(GameState::Inspecting))),
            );
    }
}
