use bevy::prelude::*;

/// Marker component for the player-controlled drone entity.
#[derive(Component, Reflect)]
pub struct Player;

/// Set to `true` on frames where the cursor was warped back to center,
/// so [`super::systems::fly`] can discard any synthetic mouse-motion delta.
#[derive(Resource, Default)]
pub struct CursorRecentered(pub bool);
