//! ECS integration tests for drone startup and runtime systems.

use std::time::Duration;

use bevy::animation::AnimationPlugin;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

use super::DroneConfig;
use super::entities::{CursorRecentered, Elbow, LaserPipe, LaserRay, Player};
use super::systems;
use crate::h_terrain::InSight;
use crate::intro::IntroConfig;
use crate::{GameState, GroundLevel, PlayerMoved, PlayerPos};

/// Builds a test app that goes through the full Intro → Arming → Running lifecycle.
///
/// Returns the app in `GameState::Running` with animations completed and stopped,
/// so `aim_pipe` and `fly` have full control of transforms.
fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(StatesPlugin)
        .add_plugins(bevy::transform::TransformPlugin)
        .add_plugins(AnimationPlugin)
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(DroneConfig::default())
        .insert_resource(IntroConfig {
            // Short durations so we can tick through quickly
            tilt_up_duration: 0.1,
            highlight_delay: 0.1,
            tilt_down_duration: 0.1,
            tilt_down_angle: 10.0,
        })
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

    app.add_systems(Startup, systems::create_drone_materials);
    app.add_systems(
        Startup,
        systems::spawn_drone.after(systems::create_drone_materials),
    );
    app.add_systems(
        Startup,
        systems::link_elbow_animation.after(systems::spawn_drone),
    );
    app.add_systems(OnEnter(GameState::Arming), systems::start_arming);
    app.add_systems(
        Update,
        (
            systems::fly,
            systems::aim_pipe,
            systems::fire_laser.after(systems::aim_pipe),
        )
            .run_if(in_state(GameState::Running)),
    );

    // Startup (spawns entities, starts intro animation)
    app.update();

    // Tick through Intro (0.3s total at 100ms/tick = 3-4 ticks + margin)
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Arming,
        "Should have transitioned to Arming after intro"
    );

    // Tick through Arming (0.6s default at 100ms/tick = 6-7 ticks + margin)
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Running,
        "Should have transitioned to Running after arming"
    );

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
    let elbow = w.query_filtered::<Entity, With<Elbow>>().single(w).unwrap();
    let pipe = w
        .query_filtered::<Entity, With<LaserPipe>>()
        .single(w)
        .unwrap();

    let player_children = w
        .entity(player)
        .get::<Children>()
        .expect("Player should have Children");
    assert!(
        player_children.iter().any(|c| c == elbow),
        "Elbow should be a child of Player"
    );

    let elbow_children = w
        .entity(elbow)
        .get::<Children>()
        .expect("Elbow should have Children");
    assert!(
        elbow_children.iter().any(|c| c == pipe),
        "LaserPipe should be a child of Elbow"
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

    // Spawn InSight target; aim_pipe will rotate elbow toward it.
    let target_pos = Vec3::new(5.0, 0.0, 5.0);
    app.world_mut()
        .spawn((InSight, Transform::from_translation(target_pos)));
    app.update();

    // Read pipe GlobalTransform AFTER aim_pipe has rotated elbow.
    let pipe_gt = {
        let w = app.world_mut();
        *w.query_filtered::<&GlobalTransform, With<LaserPipe>>()
            .single(w)
            .unwrap()
    };

    let front = pipe_gt.transform_point(Vec3::NEG_Y * half_h);
    let back = pipe_gt.transform_point(Vec3::Y * half_h);

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

// ── Intro animation rotation ────────────────────────────────────

#[test]
fn intro_animation_tilts_camera() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(StatesPlugin)
        .add_plugins(bevy::transform::TransformPlugin)
        .add_plugins(AnimationPlugin)
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(DroneConfig::default())
        .insert_resource(IntroConfig {
            tilt_up_duration: 0.3,
            highlight_delay: 0.1,
            tilt_down_duration: 0.1,
            tilt_down_angle: 10.0,
        })
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

    app.add_systems(Startup, systems::create_drone_materials);
    app.add_systems(
        Startup,
        systems::spawn_drone.after(systems::create_drone_materials),
    );
    app.add_systems(
        Startup,
        systems::link_elbow_animation.after(systems::spawn_drone),
    );

    // Startup
    app.update();

    // Capture initial pitch (looking down at (5,0,5) from (0,2,0))
    let initial_pitch = {
        let w = app.world_mut();
        let rot = w
            .query_filtered::<&Transform, With<Player>>()
            .single(w)
            .unwrap()
            .rotation;
        let (_, pitch, _) = rot.to_euler(EulerRot::YXZ);
        pitch
    };
    assert!(
        initial_pitch < 0.0,
        "Initial pitch should be negative (looking down), got {initial_pitch}"
    );

    // Tick 2 frames — animation should tilt camera toward horizontal
    app.update();
    app.update();

    let after_pitch = {
        let w = app.world_mut();
        let rot = w
            .query_filtered::<&Transform, With<Player>>()
            .single(w)
            .unwrap()
            .rotation;
        let (_, pitch, _) = rot.to_euler(EulerRot::YXZ);
        pitch
    };
    assert!(
        after_pitch > initial_pitch,
        "Pitch should increase (tilt toward horizontal) during intro: \
         initial={initial_pitch} after={after_pitch}"
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

// ── Arming animation ────────────────────────────────────────────

#[test]
fn arming_animation_reaches_armed_rotation() {
    // test_app() already runs through Intro → Arming → Running.
    // Just verify elbow ended up at armed rotation.
    let mut app = test_app();
    let armed = systems::armed_quat();

    let w = app.world_mut();
    let rot = w
        .query_filtered::<&Transform, With<Elbow>>()
        .single(w)
        .unwrap()
        .rotation;
    let angle = rot.angle_between(armed);
    assert!(
        angle < 0.05,
        "Elbow should be at armed rotation after arming completes (angle={angle})"
    );
}

// ── Aim pipe tracking ──────────────────────────────────────────

#[test]
fn aim_pipe_tracks_target() {
    let mut app = test_app();

    let target_pos = Vec3::new(5.0, 0.0, 5.0);
    app.world_mut()
        .spawn((InSight, Transform::from_translation(target_pos)));
    app.update(); // propagate InSight's GlobalTransform

    // Slerp converges over several frames (ease-out)
    for _ in 0..20 {
        app.update();
    }

    let w = app.world_mut();
    let elbow_gt = w
        .query_filtered::<&GlobalTransform, With<Elbow>>()
        .single(w)
        .unwrap();

    let elbow_world = elbow_gt.translation();
    let expected_dir = (target_pos - elbow_world).normalize();

    // Elbow's local -Y in world space should point toward the target
    let local_neg_y_world = (elbow_gt.transform_point(Vec3::NEG_Y) - elbow_world).normalize();
    let dot = local_neg_y_world.dot(expected_dir);
    assert!(
        dot > 0.99,
        "Elbow -Y should point toward target (dot={dot})"
    );
}

#[test]
fn aim_pipe_eases_toward_target() {
    let mut app = test_app();
    let armed = systems::armed_quat();

    // Use realistic frame rate so aim_speed * dt < 1.0
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));

    let target_pos = Vec3::new(5.0, 0.0, 5.0);
    app.world_mut()
        .spawn((InSight, Transform::from_translation(target_pos)));
    app.update(); // propagate GlobalTransform

    // After one aim_pipe tick the elbow should have started moving but not arrived
    app.update();
    let rot_after_one = {
        let w = app.world_mut();
        w.query_filtered::<&Transform, With<Elbow>>()
            .single(w)
            .unwrap()
            .rotation
    };
    let angle_from_armed = rot_after_one.angle_between(armed);
    assert!(
        angle_from_armed > 0.01,
        "Elbow should have started moving away from armed (angle={angle_from_armed})"
    );

    // Compute the goal rotation for comparison
    let goal = {
        let w = app.world_mut();
        let elbow_pos = w
            .query_filtered::<&GlobalTransform, With<Elbow>>()
            .single(w)
            .unwrap()
            .translation();
        let player_rot = w
            .query_filtered::<&GlobalTransform, With<Player>>()
            .single(w)
            .unwrap()
            .to_scale_rotation_translation()
            .1;
        let dir = (target_pos - elbow_pos).normalize();
        Quat::from_rotation_arc(Vec3::NEG_Y, player_rot.inverse() * dir)
    };
    let angle_from_goal = rot_after_one.angle_between(goal);
    assert!(
        angle_from_goal > 0.01,
        "Elbow should not have reached goal yet after one frame (angle={angle_from_goal})"
    );
}

#[test]
fn aim_pipe_eases_back() {
    let mut app = test_app();
    let armed = systems::armed_quat();

    // Spawn target and let aim_pipe converge toward it
    let target = app
        .world_mut()
        .spawn((
            InSight,
            Transform::from_translation(Vec3::new(5.0, 0.0, 5.0)),
        ))
        .id();
    for _ in 0..20 {
        app.update();
    }

    // Verify rotation moved away from armed
    {
        let w = app.world_mut();
        let rot = w
            .query_filtered::<&Transform, With<Elbow>>()
            .single(w)
            .unwrap()
            .rotation;
        let angle = rot.angle_between(armed);
        assert!(
            angle > 0.01,
            "Elbow should have rotated away from armed (angle={angle})"
        );
    }

    // Remove target and let aim_pipe ease back over several frames
    app.world_mut().despawn(target);
    for _ in 0..20 {
        app.update();
    }

    // Verify eased back close to armed
    {
        let w = app.world_mut();
        let rot = w
            .query_filtered::<&Transform, With<Elbow>>()
            .single(w)
            .unwrap()
            .rotation;
        let angle = rot.angle_between(armed);
        assert!(
            angle < 0.01,
            "Elbow should have eased back to armed rotation (angle={angle})"
        );
    }
}
