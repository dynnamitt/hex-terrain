//! Runtime systems for height-based terrain.

use bevy::prelude::*;

use super::entities::HGrid;
use crate::{PlayerMoved, PlayerPos};

/// Sets `PlayerPos.pos.y` from terrain interpolation.
/// Skipped when [`PlayerMoved`] is `false` (no xz/altitude change this frame).
pub fn update_player_height(
    grid: Single<&HGrid>,
    mut player: ResMut<PlayerPos>,
    mut moved: ResMut<PlayerMoved>,
) {
    if !moved.0 {
        return;
    }
    moved.0 = false;
    let xz = Vec2::new(player.pos.x, player.pos.z);
    player.pos.y = grid.terrain.interpolate_height(xz) + player.altitude;
}

/// Seeds [`PlayerPos`] from the camera transform left by the intro sequence.
///
/// Sets xz position, derives altitude from terrain height, and computes the
/// initial `pos.y` so the first `fly()` frame doesn't snap to the origin.
pub fn sync_initial_altitude(
    grid: Single<&HGrid>,
    mut player: ResMut<PlayerPos>,
    mut moved: ResMut<PlayerMoved>,
    cam_tf: Single<&Transform, With<crate::drone::Player>>,
) {
    let xz = Vec2::new(cam_tf.translation.x, cam_tf.translation.z);
    let terrain_h = grid.terrain.interpolate_height(xz);
    player.pos.x = cam_tf.translation.x;
    player.pos.z = cam_tf.translation.z;
    player.altitude = cam_tf.translation.y - terrain_h;
    player.pos.y = cam_tf.translation.y;
    moved.0 = true;
}
