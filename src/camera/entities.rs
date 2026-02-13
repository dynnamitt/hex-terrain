use bevy::prelude::*;
use hexx::Hex;

/// Marker component for the player-controlled camera entity.
#[derive(Component, Reflect)]
pub struct TerrainCamera;

/// Tracks which hex cell the camera currently occupies.
#[derive(Resource, Default)]
pub struct CameraCell {
    /// Hex coordinate directly below the camera.
    pub current: Hex,
    /// The cell the camera occupied last frame (if it moved).
    pub previous: Option<Hex>,
    /// `true` for exactly one frame after a cell transition.
    pub changed: bool,
}

/// Dynamic vertical offset above terrain, adjusted by Q/E keys.
#[derive(Resource)]
pub struct CameraAltitude(pub f32);

/// Set to `true` on frames where the cursor was warped back to center,
/// so [`super::systems::move_camera`] can discard any synthetic mouse-motion delta.
#[derive(Resource, Default)]
pub struct CursorRecentered(pub bool);
