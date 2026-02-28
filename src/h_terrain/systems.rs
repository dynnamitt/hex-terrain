//! Runtime systems for height-based terrain.

use bevy::prelude::*;

use super::entities::HGrid;
use crate::PlayerPos;

/// Sets `PlayerPos.pos.y` from terrain interpolation.
pub fn update_player_height(grid_q: Query<&HGrid>, mut player: ResMut<PlayerPos>) {
    let Ok(grid) = grid_q.single() else { return };
    let xz = Vec2::new(player.pos.x, player.pos.z);
    player.pos.y = grid.terrain.interpolate_height(xz) + player.altitude;
}

/// Syncs [`PlayerPos::altitude`] from the camera's current Y on enter Running.
pub fn sync_initial_altitude(
    grid_q: Query<&HGrid>,
    mut player: ResMut<PlayerPos>,
    cam_q: Query<&Transform, With<crate::drone::Player>>,
) {
    let Ok(grid) = grid_q.single() else { return };
    let Ok(cam_tf) = cam_q.single() else { return };
    let xz = Vec2::new(cam_tf.translation.x, cam_tf.translation.z);
    let terrain_h = grid.terrain.interpolate_height(xz);
    player.altitude = cam_tf.translation.y - terrain_h;
}
