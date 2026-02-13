//! Hex grid generation: noise heights, per-hex radii, and vertex positions.
//!
//! Spawns the [`HexGrid`] entity at startup using Perlin-based fractal noise
//! for terrain heights and per-hex radii. All [`HexSunDisc`](crate::petals::HexSunDisc)
//! entities are children of the grid entity; petal geometry is handled by [`crate::petals`].

mod entities;
mod systems;

pub use entities::HexGrid;

use bevy::prelude::*;

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
        app.register_type::<GridConfig>()
            .insert_resource(self.0.clone())
            .add_systems(
                Startup,
                systems::generate_grid.after(crate::visuals::setup_visuals),
            )
            .add_systems(Update, systems::fade_nearby_poles);

        #[cfg(debug_assertions)]
        app.add_systems(Update, systems::draw_hex_labels);
    }
}
