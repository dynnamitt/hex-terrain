//! Hex terrain: grid generation, petal spawning, height interpolation.
//!
//! Merges the former `grid`, `petals`, and `visuals` modules into a single
//! terrain plugin with nested config.

mod entities;
mod systems;
mod terrain_hex_layout;

pub use entities::{HexGrid, HexSunDisc};

use bevy::prelude::*;

use crate::GameState;

/// Nested configuration for the terrain subsystem.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct TerrainConfig {
    /// Grid generation settings.
    pub grid: GridSettings,
    /// Flower geometry: pole, petal edge, and hex-radius settings.
    pub flower: FlowerSettings,
    /// Background clear color.
    pub clear_color: Color,
}

/// Grid layout and noise parameters.
#[derive(Clone, Debug, Reflect)]
pub struct GridSettings {
    /// Number of hex rings around the origin (~1200 hexes at 20).
    pub radius: u32,
    /// Distance in world-units between adjacent hex centers.
    pub point_spacing: f32,
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
    /// Maximum terrain elevation produced by the noise function.
    pub max_height: f32,
    /// Smallest visual hex radius (noise-derived per cell).
    pub min_hex_radius: f32,
    /// Largest visual hex radius (noise-derived per cell).
    pub max_hex_radius: f32,
}

/// Flower geometry: pole dimensions, and edge/face spawning.
#[derive(Clone, Debug, Reflect)]
pub struct FlowerSettings {
    /// Pole cylinder radius as a fraction of the hex's visual radius.
    pub pole_radius_factor: f32,
    /// Distance at which poles reach full opacity.
    pub pole_fade_distance: f32,
    /// Minimum alpha when the camera is right on top of a pole.
    pub pole_min_alpha: f32,
    /// Gap between pole top and hex face.
    pub pole_gap: f32,
    /// Thickness of edge line cuboids.
    pub edge_thickness: f32,
    /// How many hex rings around the drone to reveal per cell transition.
    pub reveal_radius: u32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            grid: GridSettings {
                radius: 20,
                point_spacing: 4.0,
                height_noise_seed: 42,
                radius_noise_seed: 137,
                height_noise_octaves: 4,
                radius_noise_octaves: 3,
                height_noise_scale: 50.0,
                radius_noise_scale: 30.0,
                max_height: 10.0,
                min_hex_radius: 0.2,
                max_hex_radius: 2.6,
            },
            flower: FlowerSettings {
                pole_radius_factor: 0.06,
                pole_fade_distance: 40.0,
                pole_min_alpha: 0.05,
                pole_gap: 0.05,
                edge_thickness: 0.03,
                reveal_radius: 2,
            },
            clear_color: Color::srgb(0.01, 0.01, 0.02),
        }
    }
}

/// Terrain plugin: grid generation at startup, petal spawning at runtime.
pub struct TerrainPlugin(pub TerrainConfig);

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TerrainConfig>()
            .register_type::<entities::HeightPole>()
            .register_type::<HexSunDisc>()
            .register_type::<entities::QuadLeaf>()
            .register_type::<entities::TriLeaf>()
            .register_type::<entities::PetalEdge>()
            .insert_resource(self.0.clone())
            .insert_resource(ClearColor(self.0.clear_color))
            .init_resource::<entities::DrawnCells>()
            .add_systems(Startup, systems::generate_grid)
            .add_systems(
                Update,
                systems::update_player_height.run_if(in_state(GameState::Running)),
            )
            .add_systems(
                Update,
                systems::track_active_hex
                    .after(systems::update_player_height)
                    .run_if(in_state(GameState::Running).or(in_state(GameState::Intro))),
            )
            .add_systems(
                Update,
                systems::spawn_petals
                    .after(systems::track_active_hex)
                    .run_if(any_with_component::<HexGrid>)
                    .run_if(in_state(GameState::Running)),
            )
            .add_systems(Update, systems::highlight_nearby_poles);

        app.add_systems(
            Update,
            systems::draw_hex_labels.run_if(in_state(GameState::Debugging)),
        );
    }
}
