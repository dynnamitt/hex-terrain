use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

use super::DroneConfig;
use crate::{PlayerMoved, PlayerPos};

/// Marker component for the player-controlled drone entity.
#[derive(Component, Reflect)]
pub struct Player;

/// Set to `true` on frames where the cursor was warped back to center,
/// so [`super::systems::fly`] can discard any synthetic mouse-motion delta.
#[derive(Resource, Default)]
pub struct CursorRecentered(pub bool);

/// Bundled system parameters for the drone flight system.
#[derive(SystemParam)]
pub struct DroneInput<'w, 's> {
    pub time: Res<'w, Time>,
    pub keys: Res<'w, ButtonInput<KeyCode>>,
    pub mouse_motion: MessageReader<'w, 's, MouseMotion>,
    pub scroll: MessageReader<'w, 's, MouseWheel>,
    pub recentered: Res<'w, CursorRecentered>,
    pub cfg: Res<'w, DroneConfig>,
    pub player: ResMut<'w, PlayerPos>,
    pub moved: ResMut<'w, PlayerMoved>,
}
