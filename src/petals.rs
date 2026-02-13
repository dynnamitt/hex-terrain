//! Petal-based entity hierarchy for hex gap geometry.
//!
//! Replaces the flat edge/gap-face spawning with a parent–child model:
//! `HexSunDisc` → `QuadLeaf`/`TriLeaf` → `PetalEdge`, enabling future
//! reactive height updates via entity references.

mod entities;
mod systems;

pub use entities::{HeightPole, HexEntities, HexSunDisc, QuadLeaf, TriLeaf};

use bevy::prelude::*;

use crate::grid::HexGrid;

/// Per-plugin configuration for petal spawning.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct PetalsConfig {
    /// Thickness of edge line cuboids.
    pub edge_thickness: f32,
    /// How many hex rings around the camera to reveal per cell transition.
    pub reveal_radius: u32,
}

impl Default for PetalsConfig {
    fn default() -> Self {
        Self {
            edge_thickness: 0.03,
            reveal_radius: 2,
        }
    }
}

/// Progressive petal spawning as the camera reveals new cells.
pub struct PetalsPlugin(pub PetalsConfig);

impl Plugin for PetalsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightPole>()
            .register_type::<HexSunDisc>()
            .register_type::<QuadLeaf>()
            .register_type::<TriLeaf>()
            .register_type::<entities::PetalEdge>()
            .register_type::<PetalsConfig>()
            .insert_resource(self.0.clone())
            .init_resource::<entities::DrawnCells>()
            .add_systems(
                Update,
                systems::spawn_petals
                    .after(crate::camera::track_camera_cell)
                    .run_if(any_with_component::<HexGrid>),
            );
    }
}
