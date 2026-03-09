use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::MouseScrollUnit;
use bevy::post_process::bloom::{Bloom, BloomCompositeMode};
use bevy::prelude::*;
use bevy::render::view::Hdr;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::WindowFocused;
use bevy::window::{CursorGrabMode, CursorOptions};

use bevy_egui::egui;

use super::DroneConfig;
#[cfg(not(target_arch = "wasm32"))]
use super::entities::CursorRecentered;
use super::entities::{ArmingTimer, DroneInput, Elbow, LaserPipe, LaserRay, Player};
use super::materials::DroneMaterials;
use crate::h_terrain::{InSight, edge_cuboid_transform};
use crate::math;

/// Creates and inserts the [`DroneMaterials`] resource.
pub fn create_drone_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DroneMaterials::new(&mut materials));
}

/// Spawns the Camera3d entity with Player marker, HDR, and bloom.
///
/// Must run after terrain seed so that [`GroundLevel`] is `Some`.
pub fn spawn_drone(
    mut commands: Commands,
    cfg: Res<DroneConfig>,
    mut player: ResMut<crate::PlayerPos>,
    ground: Res<crate::GroundLevel>,
    mut moved: ResMut<crate::PlayerMoved>,
    mut meshes: ResMut<Assets<Mesh>>,
    drone_mats: Res<DroneMaterials>,
) {
    let ground_y = ground.0.unwrap_or(0.0);
    player.offset = cfg.lowest_offset;
    moved.0 = true;
    let spawn_y = ground_y + cfg.lowest_offset;
    commands
        .spawn((
            Name::new("Player"),
            Camera3d::default(),
            Hdr,
            Tonemapping::TonyMcMapface,
            Bloom {
                intensity: cfg.bloom_intensity,
                composite_mode: BloomCompositeMode::Additive,
                ..Bloom::NATURAL
            },
            Transform::from_xyz(0.0, spawn_y, 0.0)
                .looking_at(Vec3::new(5.0, ground_y, 5.0), Vec3::Y),
            Player,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Elbow"),
                    Elbow,
                    ArmingTimer(0.0),
                    Visibility::default(),
                    Transform::from_translation(cfg.pipe_offset).with_rotation(
                        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
                            * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    ),
                ))
                .with_children(|elbow| {
                    elbow.spawn((
                        Name::new("LaserPipe"),
                        LaserPipe,
                        Mesh3d(meshes.add(Cylinder::new(cfg.pipe_radius, cfg.pipe_length / 2.0))),
                        MeshMaterial3d(drone_mats.pipe.clone()),
                        Transform::from_translation(Vec3::NEG_Y * (cfg.pipe_length / 4.0)),
                    ));
                });
        });

    // Laser ray as root entity (world-space positioning)
    commands.spawn((
        Name::new("LaserRay"),
        LaserRay,
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(drone_mats.laser_ray.clone()),
        Transform::default(),
        Visibility::Hidden,
    ));
}

/// Animates the laser pipe from its hidden rotation into the armed (forward-facing) position.
pub fn arm_pipe(
    time: Res<Time>,
    cfg: Res<DroneConfig>,
    mut elbow_q: Single<(&mut Transform, &mut ArmingTimer), With<Elbow>>,
    mut next_state: ResMut<NextState<crate::GameState>>,
) {
    let (tf, timer) = &mut *elbow_q;
    timer.0 += time.delta_secs();
    let t = (timer.0 / cfg.arm_duration).min(1.0);
    let eased = crate::math::ease_out_cubic(t);

    let hidden = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let armed = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    tf.rotation = hidden.slerp(armed, eased);

    if t >= 1.0 {
        next_state.set(crate::GameState::Running);
    }
}

/// WASD + mouse look + Q/E/scroll offset. Writes to [`PlayerPos`].
pub fn fly(mut input: DroneInput, mut transform: Single<&mut Transform, With<Player>>) {
    // Mouse look: yaw (horizontal) + pitch (vertical)
    let mut yaw = 0.0;
    let mut pitch = 0.0;
    if input.recentered.0 {
        for _ in input.mouse_motion.read() {}
    } else {
        for ev in input.mouse_motion.read() {
            yaw -= ev.delta.x * input.cfg.mouse_sensitivity_x;
            pitch -= ev.delta.y * input.cfg.mouse_sensitivity_y;
        }
    }
    if yaw != 0.0 {
        transform.rotate_y(yaw);
    }
    if pitch != 0.0 {
        let (_, current_pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let pitch_delta = math::clamp_pitch(current_pitch, pitch, input.cfg.pitch_margin);
        transform.rotate_local_x(pitch_delta);
    }

    // WASD movement in the drone's forward/right plane (XZ only)
    let forward = transform.forward();
    let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right = transform.right();
    let right_xz = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    let mut direction = Vec3::ZERO;
    if input.keys.pressed(KeyCode::KeyW) {
        direction += forward_xz;
    }
    if input.keys.pressed(KeyCode::KeyS) {
        direction -= forward_xz;
    }
    if input.keys.pressed(KeyCode::KeyD) {
        direction += right_xz;
    }
    if input.keys.pressed(KeyCode::KeyA) {
        direction -= right_xz;
    }

    if direction != Vec3::ZERO {
        direction = direction.normalize();
        let delta = direction * input.cfg.move_speed * input.time.delta_secs();
        input.player.xz.x += delta.x;
        input.player.xz.y += delta.z;
        input.moved.0 = true;
    }

    // Q/E vertical offset adjustment
    if input.keys.pressed(KeyCode::KeyE) {
        input.player.offset += input.cfg.move_speed * input.time.delta_secs();
        input.moved.0 = true;
    }
    if input.keys.pressed(KeyCode::KeyQ) {
        input.player.offset -= input.cfg.move_speed * input.time.delta_secs();
        input.moved.0 = true;
    }

    // Mouse scroll also adjusts offset
    for ev in input.scroll.read() {
        let lines = match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 40.0,
        };
        input.player.offset += lines * input.cfg.scroll_sensitivity;
        input.moved.0 = true;
    }

    // Clamp offset to lowest_offset floor
    input.player.offset = input.player.offset.max(input.cfg.lowest_offset);

    // Apply position from PlayerPos + GroundLevel
    let ground_y = input.ground.0.unwrap_or(0.0);
    let target_y = ground_y + input.player.offset;
    transform.translation.x = input.player.xz.x;
    transform.translation.z = input.player.xz.y;
    if target_y > transform.translation.y {
        transform.translation.y = target_y;
    } else {
        transform.translation.y += (target_y - transform.translation.y) * input.cfg.height_lerp;
    }
}

#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
pub fn recenter_cursor(
    mut window: Single<&mut Window>,
    mut focus_events: MessageReader<WindowFocused>,
    mut recentered: ResMut<CursorRecentered>,
    cfg: Res<DroneConfig>,
) {
    recentered.0 = false;

    let gained_focus = focus_events.read().any(|ev| ev.focused);
    let w = window.width();
    let h = window.height();
    let center = Vec2::new(w / 2.0, h / 2.0);

    if gained_focus {
        window.set_cursor_position(Some(center));
        recentered.0 = true;
        return;
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

/// Re-locks the cursor on left click (WASM only).
///
/// Browsers release pointer lock when the user presses Escape or the window
/// loses focus. This system re-engages the lock on the next left click,
/// which counts as a user gesture the browser requires.
#[cfg(target_arch = "wasm32")]
pub fn lock_cursor_on_click(
    mouse: Res<ButtonInput<MouseButton>>,
    mut q: Query<(&mut CursorOptions, &mut Window)>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        for (mut opts, mut window) in &mut q {
            opts.visible = false;
            opts.grab_mode = CursorGrabMode::Confined;
            let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
            window.set_cursor_position(Some(center));
        }
    }
}

/// Draws a small white crosshair at screen center so the player can see the aim point.
pub fn draw_crosshair(mut egui_ctx: Single<&mut bevy_egui::EguiContext>, window: Single<&Window>) {
    let cx = window.width() / 2.0;
    let cy = window.height() / 2.0;
    let half = 8.0;
    let stroke = egui::Stroke::new(1.5, egui::Color32::WHITE);

    egui::Area::new(egui::Id::new("crosshair"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .interactable(false)
        .show(egui_ctx.get_mut(), |ui| {
            let painter = ui.painter();
            painter.line_segment(
                [egui::pos2(cx - half, cy), egui::pos2(cx + half, cy)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(cx, cy - half), egui::pos2(cx, cy + half)],
                stroke,
            );
        });
}

/// Shows a red laser ray from the pipe tip to the aimed hex face on Space or Left Click.
pub fn fire_laser(
    pipe_q: Single<&GlobalTransform, With<LaserPipe>>,
    mut ray_q: Single<(&mut Transform, &mut Visibility), With<LaserRay>>,
    sight_target: Query<&GlobalTransform, With<InSight>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cfg: Res<DroneConfig>,
) {
    let firing = keys.pressed(KeyCode::Space) || mouse.pressed(MouseButton::Left);
    let (ray_tf, ray_vis) = &mut *ray_q;

    if !firing {
        *ray_vis.as_mut() = Visibility::Hidden;
        return;
    }

    let Ok(target_gt) = sight_target.single() else {
        *ray_vis.as_mut() = Visibility::Hidden;
        return;
    };

    let tip = pipe_q.transform_point(Vec3::NEG_Y * (cfg.pipe_length / 4.0));
    let target = target_gt.translation();

    let (midpoint, length, rotation) = edge_cuboid_transform(tip, target);
    ray_tf.translation = midpoint;
    ray_tf.rotation = rotation;
    ray_tf.scale = Vec3::new(length, cfg.laser_thickness, cfg.laser_thickness);
    *ray_vis.as_mut() = Visibility::Visible;
}
