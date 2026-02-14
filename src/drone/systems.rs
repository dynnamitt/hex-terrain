use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::post_process::bloom::{Bloom, BloomCompositeMode};
use bevy::prelude::*;
use bevy::render::view::Hdr;
use bevy::window::{CursorGrabMode, CursorOptions, WindowFocused};

use super::DroneConfig;
use super::entities::{CursorRecentered, Player};
use crate::PlayerPos;
use crate::math;

/// Spawns the Camera3d entity with Player marker, HDR, and bloom.
pub fn spawn_drone(mut commands: Commands, cfg: Res<DroneConfig>) {
    commands.spawn((
        Name::new("Player"),
        Camera3d::default(),
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: cfg.bloom_intensity,
            composite_mode: BloomCompositeMode::Additive,
            ..Bloom::NATURAL
        },
        Transform::from_xyz(0.0, 12.0, 0.0).looking_at(Vec3::new(5.0, 0.0, 5.0), Vec3::Y),
        Player,
    ));
}

/// WASD + mouse look + Q/E/scroll altitude. Writes to [`PlayerPos`].
#[allow(clippy::too_many_arguments)]
pub fn fly(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut scroll: MessageReader<MouseWheel>,
    mut query: Query<&mut Transform, With<Player>>,
    recentered: Res<CursorRecentered>,
    cfg: Res<DroneConfig>,
    mut player: ResMut<PlayerPos>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    // Mouse look: yaw (horizontal) + pitch (vertical)
    let mut yaw = 0.0;
    let mut pitch = 0.0;
    if recentered.0 {
        for _ in mouse_motion.read() {}
    } else {
        for ev in mouse_motion.read() {
            yaw -= ev.delta.x * cfg.mouse_sensitivity_x;
            pitch -= ev.delta.y * cfg.mouse_sensitivity_y;
        }
    }
    if yaw != 0.0 {
        transform.rotate_y(yaw);
    }
    if pitch != 0.0 {
        let (_, current_pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let pitch_delta = math::clamp_pitch(current_pitch, pitch, cfg.pitch_margin);
        transform.rotate_local_x(pitch_delta);
    }

    // WASD movement in the drone's forward/right plane (XZ only)
    let forward = transform.forward();
    let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right = transform.right();
    let right_xz = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        direction += forward_xz;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction -= forward_xz;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction += right_xz;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction -= right_xz;
    }

    if direction != Vec3::ZERO {
        direction = direction.normalize();
        let delta = direction * cfg.move_speed * time.delta_secs();
        player.pos.x += delta.x;
        player.pos.z += delta.z;
    }

    // Q/E vertical altitude adjustment
    if keys.pressed(KeyCode::KeyE) {
        player.altitude += cfg.move_speed * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyQ) {
        player.altitude -= cfg.move_speed * time.delta_secs();
    }

    // Mouse scroll also adjusts altitude
    for ev in scroll.read() {
        let lines = match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 40.0,
        };
        player.altitude += lines * cfg.scroll_sensitivity;
    }

    // Apply position from PlayerPos (y is set by terrain::update_player_height)
    let target_y = player.pos.y;
    transform.translation.x = player.pos.x;
    transform.translation.z = player.pos.z;
    transform.translation.y += (target_y - transform.translation.y) * cfg.height_lerp;
}

pub fn hide_cursor(mut q: Query<(&mut CursorOptions, &mut Window)>) {
    for (mut opts, mut window) in &mut q {
        opts.visible = false;
        opts.grab_mode = CursorGrabMode::Confined;
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
    }
}

/// Warps cursor back to center when it drifts near a window edge or when
/// the window regains focus.
pub fn recenter_cursor(
    mut windows: Query<&mut Window>,
    mut focus_events: MessageReader<WindowFocused>,
    mut recentered: ResMut<CursorRecentered>,
    cfg: Res<DroneConfig>,
) {
    recentered.0 = false;

    let gained_focus = focus_events.read().any(|ev| ev.focused);

    for mut window in &mut windows {
        let w = window.width();
        let h = window.height();
        let center = Vec2::new(w / 2.0, h / 2.0);

        if gained_focus {
            window.set_cursor_position(Some(center));
            recentered.0 = true;
            continue;
        }

        if let Some(pos) = window.cursor_position()
            && (pos.x < cfg.edge_margin
                || pos.x > w - cfg.edge_margin
                || pos.y < cfg.edge_margin
                || pos.y > h - cfg.edge_margin)
        {
            window.set_cursor_position(Some(center));
            recentered.0 = true;
        }
    }
}
