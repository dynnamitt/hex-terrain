//! Height-based terrain: pivot-point grid with per-hex corners.

mod entities;
mod gaps;
mod h_grid_layout;
pub(crate) mod materials;
mod math;
mod startup_systems;
mod systems;
#[cfg(test)]
mod tests;

use bevy::ecs::schedule::InternedSystemSet;
use bevy::prelude::*;

use crate::{DebugFlag, GameState};

pub use entities::InSight;
pub use materials::OrigPalette;
pub use math::edge_cuboid_transform;

/// Pipeline ordering for h_terrain update systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum HTerrainPhase {
    /// Sets [`GroundLevel`](crate::GroundLevel) from terrain interpolation.
    UpdateGround,
    /// Tags nearby [`entities::HCell`] entities with [`entities::InFov`].
    TrackFov,
    /// Swaps materials on meshes based on [`entities::InFov`] presence.
    Highlight,
    /// Raycasts screen center to tag the aimed hex face with [`InSight`].
    Sight,
}

/// Laser mining strength, controlling extraction rate and tick interval.
#[derive(Resource, Reflect)]
pub struct LaserStrength {
    /// Numeric upgrade tier.
    pub level: u8,
    /// Y units to lower per extraction tick.
    pub extract_height: f32,
    /// Seconds between extraction ticks.
    pub extraction_time: f32,
}

impl Default for LaserStrength {
    fn default() -> Self {
        Self {
            level: 1,
            extract_height: 0.2,
            extraction_time: 0.5,
        }
    }
}

/// Configuration for the height-based terrain subsystem.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct HTerrainConfig {
    /// Grid generation settings.
    pub grid: HGridSettings,
    /// Background clear color.
    pub clear_color: Color,
    /// Duration of the fov highlight fade in seconds.
    pub fov_transition_secs: f32,
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
                height_noise_seed: 43,
                radius_noise_seed: 137,
                height_noise_octaves: 4,
                radius_noise_octaves: 3,
                height_noise_scale: 50.0,
                radius_noise_scale: 30.0,
                max_height: 20.0,
                min_hex_radius: 0.2,
                max_hex_radius: 2.6,
            },
            clear_color: OrigPalette::ClearColor.into(),
            fov_transition_secs: 0.5,
        }
    }
}

/// Height-based terrain plugin.
pub struct HTerrainPlugin {
    /// Terrain configuration.
    pub config: HTerrainConfig,
    /// Optional system set that player-height updates must run after.
    pub after_player_movement: Option<InternedSystemSet>,
    /// Optional Startup set to place `seed_ground_level` in (for ordering).
    pub terrain_seeded_set: Option<InternedSystemSet>,
}

impl Plugin for HTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LaserStrength>()
            .register_type::<LaserStrength>()
            .register_type::<HTerrainConfig>()
            .register_type::<entities::HCell>()
            .register_type::<entities::Corner>()
            .register_type::<entities::QuadOwner>()
            .register_type::<entities::QuadPos1Emitter>()
            .register_type::<entities::QuadPos2Emitter>()
            .register_type::<entities::QuadTail>()
            .register_type::<entities::TriOwner>()
            .register_type::<entities::TriPos1Emitter>()
            .register_type::<entities::TriPos2Emitter>()
            .register_type::<entities::Quad>()
            .register_type::<entities::QuadEdge>()
            .register_type::<entities::Tri>()
            .register_type::<entities::InFov>()
            .register_type::<entities::HexFace>()
            .register_type::<entities::FovTransition>()
            .register_type::<entities::InSight>()
            .register_type::<entities::PreSightMaterial>()
            .register_type::<entities::AimStar>()
            .insert_resource(self.config.clone())
            .insert_resource(ClearColor(self.config.clear_color))
            .configure_sets(
                Update,
                (
                    HTerrainPhase::UpdateGround,
                    HTerrainPhase::TrackFov.after(HTerrainPhase::UpdateGround),
                    HTerrainPhase::Highlight.after(HTerrainPhase::TrackFov),
                    HTerrainPhase::Sight.after(HTerrainPhase::Highlight),
                ),
            )
            .add_systems(Startup, startup_systems::generate_h_grid)
            .add_systems(
                Startup,
                startup_systems::verify_gap_counts
                    .after(startup_systems::generate_h_grid)
                    .run_if(|f: Res<DebugFlag>| f.0),
            );

        {
            let seed = startup_systems::seed_ground_level.after(startup_systems::generate_h_grid);
            if let Some(set) = self.terrain_seeded_set {
                app.add_systems(Startup, seed.in_set(set));
            } else {
                app.add_systems(Startup, seed);
            }
        }

        if let Some(movement_set) = self.after_player_movement {
            app.configure_sets(Update, HTerrainPhase::UpdateGround.after(movement_set));
        }

        app.add_systems(
            Update,
            (
                systems::update_ground_level.in_set(HTerrainPhase::UpdateGround),
                systems::track_player_fov.in_set(HTerrainPhase::TrackFov),
                materials::start_fov_transitions.in_set(HTerrainPhase::Highlight),
                materials::animate_fov_transitions
                    .after(HTerrainPhase::Highlight)
                    .before(HTerrainPhase::Sight),
                materials::track_in_sight.in_set(HTerrainPhase::Sight),
                systems::extract_ore.after(HTerrainPhase::Sight),
            )
                .run_if(in_state(GameState::Running)),
        );
    }
}
