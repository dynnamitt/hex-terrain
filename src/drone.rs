//! First-person drone controller.
//!
//! WASD + mouse look + Q/E/scroll altitude. Writes to [`PlayerPos`](crate::PlayerPos)
//! for terrain to consume. Spawns the Camera3d entity with bloom.

mod entities;
pub(crate) mod materials;
pub(crate) mod systems;
#[cfg(test)]
mod tests;

pub use entities::Player;

use bevy::ecs::schedule::InternedSystemSet;
use bevy::prelude::*;

use crate::GameState;
use crate::h_terrain::HTerrainPhase;

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
    /// Local-space offset of the laser pipe from the camera.
    pub pipe_offset: Vec3,
    /// Length of the laser pipe cylinder.
    pub pipe_length: f32,
    /// Radius of the laser pipe cylinder.
    pub pipe_radius: f32,
    /// Thickness of the laser ray cuboid.
    pub laser_thickness: f32,
    /// Aim pipe interpolation speed (higher = faster tracking).
    pub aim_speed: f32,
    /// Duration of the pipe swing-in animation (seconds).
    pub arm_duration: f32,
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
            pipe_offset: Vec3::new(-0.5, -0.5, -1.0),
            pipe_length: 3.0,
            pipe_radius: 0.07,
            laser_thickness: 0.015,
            aim_speed: 12.0,
            arm_duration: 0.6,
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
            .register_type::<entities::Elbow>()
            .register_type::<entities::LaserPipe>()
            .register_type::<entities::LaserRay>()
            .insert_resource(self.config.clone())
            .init_resource::<entities::CursorRecentered>();

        app.add_systems(Startup, systems::create_drone_materials);

        if let Some(set) = self.after_terrain_seed {
            app.add_systems(
                Startup,
                systems::spawn_drone
                    .after(systems::create_drone_materials)
                    .after(set),
            );
        } else {
            app.add_systems(
                Startup,
                systems::spawn_drone.after(systems::create_drone_materials),
            );
        }

        // Link Elbow's AnimatedBy after spawn_drone has run
        app.add_systems(
            Startup,
            systems::link_elbow_animation.after(systems::spawn_drone),
        );

        // Start arming animation on state enter
        app.add_systems(OnEnter(GameState::Arming), systems::start_arming);

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

        app.add_systems(
            Update,
            systems::draw_crosshair.run_if(in_state(GameState::Running)),
        )
        .add_systems(
            Update,
            (
                systems::aim_pipe.after(HTerrainPhase::Sight),
                systems::fire_laser
                    .after(HTerrainPhase::Sight)
                    .after(systems::aim_pipe),
            )
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
