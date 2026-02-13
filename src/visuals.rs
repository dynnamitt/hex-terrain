//! Scene visuals: camera, bloom, tonemapping, and shared materials.
//!
//! Sets up the `Camera3d` with HDR + bloom post-processing and creates the
//! [`ActiveNeonMaterials`] resource consumed by `grid` and `petals` when spawning
//! geometry.

mod entities;
mod systems;

pub use entities::ActiveNeonMaterials;
pub use systems::setup_visuals;

use bevy::prelude::*;

/// Per-plugin configuration for the visual setup.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct VisualsConfig {
    /// Bloom post-processing intensity.
    pub bloom_intensity: f32,
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            bloom_intensity: 0.3,
        }
    }
}

/// Sets up the camera, bloom, tonemapping, and shared neon materials.
pub struct VisualsPlugin(pub VisualsConfig);

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<VisualsConfig>()
            .insert_resource(self.0.clone())
            .add_systems(Startup, setup_visuals);
    }
}
