use bevy::animation::AnimationEvent;
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

use super::DroneConfig;
use crate::h_terrain::fov_overlay::FovMaterial;
use crate::h_terrain::materials::TerrainMaterials;
use crate::h_terrain::{AimStar, InSight};
use crate::{GroundLevel, PlayerMoved, PlayerPos};

/// Marker component for the player-controlled drone entity.
#[derive(Component, Reflect)]
pub struct Player;

/// Pivot entity between Player and LaserPipe; the pipe swings from this point.
#[derive(Component, Reflect)]
pub struct Elbow;

/// Marker on the cylindrical pipe mesh attached below-left of the camera.
#[derive(Component, Reflect)]
pub struct LaserPipe;

/// Marker on the laser ray cuboid (root entity, world-space positioned).
#[derive(Component, Reflect)]
pub struct LaserRay;

/// Animation event fired when the arming pipe swing-in animation completes.
#[derive(Clone, AnimationEvent, Reflect)]
pub struct ArmingComplete;

/// Animation event fired when the intro camera sequence completes.
#[derive(Clone, AnimationEvent, Reflect)]
pub struct IntroComplete;

/// Set to `true` on frames where the cursor was warped back to center,
/// so [`super::systems::fly`] can discard any synthetic mouse-motion delta.
#[derive(Resource, Default)]
pub struct CursorRecentered(pub bool);

/// Bundled system parameters for laser-firing visual effects.
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub struct LaserFx<'w, 's> {
    pub stars: Query<
        'w,
        's,
        &'static mut MeshMaterial3d<StandardMaterial>,
        (With<AimStar>, Without<InSight>),
    >,
    pub hex: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static MeshMaterial3d<FovMaterial>,
        ),
        (With<InSight>, Without<AimStar>),
    >,
    pub fov_assets: ResMut<'w, Assets<FovMaterial>>,
    pub mats: Res<'w, TerrainMaterials>,
}

/// Bundled system parameters for the drone flight system.
#[derive(SystemParam)]
pub struct DroneInput<'w, 's> {
    pub time: Res<'w, Time>,
    pub keys: Res<'w, ButtonInput<KeyCode>>,
    pub mouse_motion: MessageReader<'w, 's, MouseMotion>,
    pub scroll: MessageReader<'w, 's, MouseWheel>,
    pub recentered: Res<'w, CursorRecentered>,
    pub cfg: Res<'w, DroneConfig>,
    pub ground: Res<'w, GroundLevel>,
    pub player: ResMut<'w, PlayerPos>,
    pub moved: ResMut<'w, PlayerMoved>,
}
