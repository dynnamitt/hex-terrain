//! ECS integration tests for drone startup and runtime systems.

use std::time::Duration;

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

use super::DroneConfig;
use super::entities::{CursorRecentered, LaserPipe, LaserRay, Player};
use super::systems;
use crate::h_terrain::InSight;
use crate::{GameState, GroundLevel, PlayerMoved, PlayerPos};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(StatesPlugin)
        .add_plugins(bevy::transform::TransformPlugin)
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(DroneConfig::default())
        .init_resource::<PlayerPos>()
        .init_resource::<PlayerMoved>()
        .insert_resource(GroundLevel(Some(0.0)))
        .init_resource::<CursorRecentered>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_message::<MouseMotion>()
        .add_message::<MouseWheel>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )))
        .init_state::<GameState>();

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Running);

    app.add_systems(Startup, systems::create_drone_materials);
    app.add_systems(
        Startup,
        systems::spawn_drone.after(systems::create_drone_materials),
    );
    app.add_systems(
        Update,
        (systems::fly, systems::fire_laser).run_if(in_state(GameState::Running)),
    );

    app.update();
    app.update();

    app
}

// ── Startup verification ────────────────────────────────────────

#[test]
fn spawn_creates_entities() {
    let mut app = test_app();
    let w = app.world_mut();

    assert_eq!(w.query::<&Player>().iter(w).count(), 1, "exactly 1 Player");
    assert_eq!(
        w.query::<&LaserPipe>().iter(w).count(),
        1,
        "exactly 1 LaserPipe"
    );
    assert_eq!(
        w.query::<&LaserRay>().iter(w).count(),
        1,
        "exactly 1 LaserRay"
    );
}

#[test]
fn pipe_is_child_of_player() {
    let mut app = test_app();
    let w = app.world_mut();

    let player = w
        .query_filtered::<Entity, With<Player>>()
        .single(w)
        .unwrap();
    let pipe = w
        .query_filtered::<Entity, With<LaserPipe>>()
        .single(w)
        .unwrap();

    let children = w
        .entity(player)
        .get::<Children>()
        .expect("Player should have Children");
    assert!(
        children.iter().any(|c| c == pipe),
        "LaserPipe should be a child of Player"
    );
}

#[test]
fn initial_position() {
    let mut app = test_app();
    let lowest = app.world().resource::<DroneConfig>().lowest_offset;
    let w = app.world_mut();

    let tf = w
        .query_filtered::<&Transform, With<Player>>()
        .single(w)
        .unwrap();
    assert_eq!(tf.translation.x, 0.0);
    assert_eq!(tf.translation.z, 0.0);
    assert!(
        (tf.translation.y - lowest).abs() < 1e-4,
        "Player y {} should be ground(0) + lowest_offset({})",
        tf.translation.y,
        lowest,
    );
}

// ── Laser visibility ─────────────────────────────────────────────

#[test]
fn laser_hidden_when_not_firing() {
    let mut app = test_app();
    let w = app.world_mut();

    let vis = w
        .query_filtered::<&Visibility, With<LaserRay>>()
        .single(w)
        .unwrap();
    assert_eq!(*vis, Visibility::Hidden);
}

#[test]
fn laser_hidden_when_firing_without_target() {
    let mut app = test_app();

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();

    let w = app.world_mut();
    let vis = w
        .query_filtered::<&Visibility, With<LaserRay>>()
        .single(w)
        .unwrap();
    assert_eq!(*vis, Visibility::Hidden);
}

// ── Laser tip calculation (the bug test) ─────────────────────────

#[test]
fn laser_tip_at_pipe_front() {
    let mut app = test_app();
    let cfg = app.world().resource::<DroneConfig>().clone();
    let half_h = cfg.pipe_length / 4.0;

    // Read pipe GlobalTransform (propagated during PostUpdate of test_app updates).
    let pipe_gt = {
        let w = app.world_mut();
        *w.query_filtered::<&GlobalTransform, With<LaserPipe>>()
            .single(w)
            .unwrap()
    };

    let front = pipe_gt.transform_point(Vec3::NEG_Y * half_h);
    let back = pipe_gt.transform_point(Vec3::Y * half_h);

    // Spawn InSight target and let GlobalTransform propagate.
    let target_pos = Vec3::new(5.0, 0.0, 5.0);
    app.world_mut()
        .spawn((InSight, Transform::from_translation(target_pos)));
    app.update();

    // Fire laser.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();

    let w = app.world_mut();
    let ray_tf = w
        .query_filtered::<&Transform, With<LaserRay>>()
        .single(w)
        .unwrap();

    let expected_mid = (front + target_pos) / 2.0;
    let wrong_mid = (back + target_pos) / 2.0;

    let dist_ok = (ray_tf.translation - expected_mid).length();
    let dist_bad = (ray_tf.translation - wrong_mid).length();

    assert!(
        dist_ok < dist_bad,
        "Ray midpoint should be near front tip, not back.\n\
         front={front:?} back={back:?}\n\
         ray={:?} expected={expected_mid:?} wrong={wrong_mid:?}",
        ray_tf.translation,
    );
    assert!(
        dist_ok < 0.01,
        "Ray midpoint {:?} should be within 0.01 of expected {:?} (dist={dist_ok})",
        ray_tf.translation,
        expected_mid,
    );
}

// ── Fly movement ────────────────────────────────────────────────

#[test]
fn fly_wasd_updates_pos() {
    let mut app = test_app();

    app.world_mut().resource_mut::<PlayerMoved>().0 = false;
    let before = app.world().resource::<PlayerPos>().xz;

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    app.update();

    let after = app.world().resource::<PlayerPos>().xz;
    let moved = app.world().resource::<PlayerMoved>().0;

    assert!(
        (after - before).length() > 0.01,
        "PlayerPos.xz should change after W: before={before:?} after={after:?}"
    );
    assert!(moved, "PlayerMoved should be set");
}

#[test]
fn fly_offset_clamps_to_lowest() {
    let mut app = test_app();
    let lowest = app.world().resource::<DroneConfig>().lowest_offset;

    app.world_mut().resource_mut::<PlayerPos>().offset = lowest;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyQ);
    app.update();

    let offset = app.world().resource::<PlayerPos>().offset;
    assert!(
        (offset - lowest).abs() < 1e-4,
        "Offset {offset} should be clamped to lowest_offset {lowest}"
    );
}
