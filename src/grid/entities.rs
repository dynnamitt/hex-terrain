use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use hexx::Hex;

/// Central component holding the hex layout, per-cell noise data, and vertex positions.
///
/// Spawned as a single entity that parents all [`crate::petals::HexSunDisc`] entities.
#[derive(Component)]
pub struct HexGrid {
    /// Hex-to-world coordinate mapping (spacing, orientation).
    pub layout: hexx::HexLayout,
    /// Noise-derived terrain height for each hex cell.
    pub heights: HashMap<Hex, f32>,
    /// Noise-derived visual radius for each hex cell.
    pub radii: HashMap<Hex, f32>,
    /// World-space position of each hex vertex, keyed by `(hex, vertex_index 0..5)`.
    pub vertex_positions: HashMap<(Hex, u8), Vec3>,
}
