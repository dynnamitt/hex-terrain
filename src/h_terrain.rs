//! Height-based terrain: pivot-point grid with per-hex corners.

mod entities;
mod h_grid_layout;
mod startup_systems;
mod systems;

use bevy::prelude::*;

use crate::{DebugFlag, GameState};

/// Pipeline ordering for h_terrain update systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum HTerrainSet {
    /// Sets `PlayerPos.pos.y` from terrain interpolation.
    PlayerHeight,
    /// Tags nearby [`entities::HCell`] entities with [`entities::InFov`].
    TrackFov,
    /// Swaps materials on meshes based on [`entities::InFov`] presence.
    Highlight,
}

/// Configuration for the height-based terrain subsystem.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct HTerrainConfig {
    /// Grid generation settings.
    pub grid: HGridSettings,
    /// Background clear color.
    pub clear_color: Color,
}

/// Grid layout and noise parameters.
#[derive(Clone, Debug, Reflect)]
pub struct HGridSettings {
    /// Number of hex rings around the origin (~1200 hexes at 20).
    pub radius: u32,
    /// fov, tag HCell under player + this radius
    pub fov_reach: u32,
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

impl Default for HTerrainConfig {
    fn default() -> Self {
        Self {
            grid: HGridSettings {
                radius: 20,
                fov_reach: 2,
                point_spacing: 4.0,
                height_noise_seed: 42,
                radius_noise_seed: 137,
                height_noise_octaves: 4,
                radius_noise_octaves: 3,
                height_noise_scale: 50.0,
                radius_noise_scale: 30.0,
                max_height: 20.0,
                min_hex_radius: 0.2,
                max_hex_radius: 2.6,
            },
            clear_color: Color::srgb(0.01, 0.01, 0.02),
        }
    }
}

/// Height-based terrain plugin.
pub struct HTerrainPlugin(pub HTerrainConfig);

impl Plugin for HTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HTerrainConfig>()
            .register_type::<entities::HCell>()
            .register_type::<entities::Corner>()
            .register_type::<entities::QuadOwner>()
            .register_type::<entities::QuadPos2Emitter>()
            .register_type::<entities::QuadPos3Emitter>()
            .register_type::<entities::QuadTail>()
            .register_type::<entities::TriOwner>()
            .register_type::<entities::TriPos1Emitter>()
            .register_type::<entities::TriPos2Emitter>()
            .register_type::<entities::Quad>()
            .register_type::<entities::QuadEdge>()
            .register_type::<entities::Tri>()
            .register_type::<entities::InFov>()
            .register_type::<entities::HexFace>()
            .insert_resource(self.0.clone())
            .insert_resource(ClearColor(self.0.clear_color))
            .configure_sets(
                Update,
                (
                    HTerrainSet::PlayerHeight,
                    HTerrainSet::TrackFov.after(HTerrainSet::PlayerHeight),
                    HTerrainSet::Highlight.after(HTerrainSet::TrackFov),
                ),
            )
            .add_systems(Startup, startup_systems::generate_h_grid)
            .add_systems(
                Startup,
                startup_systems::verify_gap_counts
                    .after(startup_systems::generate_h_grid)
                    .run_if(|f: Res<DebugFlag>| f.0),
            )
            .add_systems(OnEnter(GameState::Running), systems::sync_initial_altitude)
            .add_systems(
                Update,
                (
                    systems::update_player_height.in_set(HTerrainSet::PlayerHeight),
                    systems::track_player_fov.in_set(HTerrainSet::TrackFov),
                    systems::highlight_fov.in_set(HTerrainSet::Highlight),
                )
                    .after(crate::drone::systems::fly)
                    .run_if(in_state(GameState::Running)),
            );
    }
}
