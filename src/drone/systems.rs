use bevy::animation::{AnimatedBy, AnimationTargetId, animated_field, prelude::*};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::MouseScrollUnit;
use bevy::math::curve::{Interval, adaptors::ConstantCurve, easing::EasingCurve};
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
use super::entities::{
    ArmingComplete, DroneInput, Elbow, IntroComplete, LaserPipe, LaserRay, Player,
};
use super::materials::DroneMaterials;
use crate::h_terrain::{InSight, edge_cuboid_transform};
use crate::intro::IntroConfig;
use crate::math;

/// Creates and inserts the [`DroneMaterials`] resource.
pub fn create_drone_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DroneMaterials::new(&mut materials));
}

/// Hidden rotation of the elbow (pipe tucked sideways).
pub fn hidden_quat() -> Quat {
    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
}

/// Armed rotation of the elbow (pipe forward-facing).
pub fn armed_quat() -> Quat {
    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
}

/// Spawns the Camera3d entity with Player marker, HDR, bloom, and animation setup.
///
/// Builds the animation graph containing intro and arming clips,
/// attaches `AnimationPlayer` + `AnimationGraphHandle` on Player,
/// and `AnimationTargetId` + `AnimatedBy` on Elbow.
///
/// Must run after terrain seed so that [`GroundLevel`] is `Some`.
#[allow(clippy::too_many_arguments)]
pub fn spawn_drone(
    mut commands: Commands,
    cfg: Res<DroneConfig>,
    intro_cfg: Res<IntroConfig>,
    mut player: ResMut<crate::PlayerPos>,
    ground: Res<crate::GroundLevel>,
    mut moved: ResMut<crate::PlayerMoved>,
    mut meshes: ResMut<Assets<Mesh>>,
    drone_mats: Res<DroneMaterials>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let ground_y = ground.0.unwrap_or(0.0);
    player.offset = cfg.lowest_offset;
    moved.0 = true;
    let spawn_y = ground_y + cfg.lowest_offset;

    // Names for animation target path resolution
    let player_name = Name::new("Player");
    let elbow_name = Name::new("Elbow");

    // ── Arming clip: eases elbow rotation hidden → armed with BackOut ──
    let mut arming_clip = AnimationClip::default();
    let elbow_target =
        AnimationTargetId::from_names([player_name.clone(), elbow_name.clone()].iter());
    let arming_curve = EasingCurve::new(hidden_quat(), armed_quat(), EaseFunction::BackOut)
        .reparametrize_linear(Interval::new(0.0, cfg.arm_duration).unwrap())
        .expect("bounded intervals");
    arming_clip.add_curve_to_target(
        elbow_target,
        AnimatableCurve::new(animated_field!(Transform::rotation), arming_curve),
    );
    arming_clip.add_event(cfg.arm_duration, ArmingComplete);

    // ── Intro clip: tilt-up → hold → tilt-down (targeting Player rotation) ──
    let player_target = AnimationTargetId::from_name(&player_name);
    let spawn_transform =
        Transform::from_xyz(0.0, spawn_y, 0.0).looking_at(Vec3::new(5.0, ground_y, 5.0), Vec3::Y);
    let (yaw, start_pitch, _) = spawn_transform.rotation.to_euler(EulerRot::YXZ);
    let horizontal = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);
    let tilt_down_rot = Quat::from_euler(
        EulerRot::YXZ,
        yaw,
        -intro_cfg.tilt_down_angle.to_radians(),
        0.0,
    );
    let start_rot = Quat::from_euler(EulerRot::YXZ, yaw, start_pitch, 0.0);

    let mut intro_clip = AnimationClip::default();
    let tilt_up_curve = EasingCurve::new(start_rot, horizontal, EaseFunction::CubicInOut)
        .reparametrize_linear(Interval::new(0.0, intro_cfg.tilt_up_duration).unwrap())
        .expect("bounded intervals");
    let hold_curve = ConstantCurve::new(
        Interval::new(0.0, intro_cfg.highlight_delay).unwrap(),
        horizontal,
    );
    let tilt_down_curve = EasingCurve::new(horizontal, tilt_down_rot, EaseFunction::CubicIn)
        .reparametrize_linear(Interval::new(0.0, intro_cfg.tilt_down_duration).unwrap())
        .expect("bounded intervals");
    let intro_rotation_curve = tilt_up_curve
        .chain(hold_curve)
        .expect("chain hold")
        .chain(tilt_down_curve)
        .expect("chain tilt-down");
    intro_clip.add_curve_to_target(
        player_target,
        AnimatableCurve::new(animated_field!(Transform::rotation), intro_rotation_curve),
    );
    let intro_total =
        intro_cfg.tilt_up_duration + intro_cfg.highlight_delay + intro_cfg.tilt_down_duration;
    intro_clip.add_event(intro_total, IntroComplete);

    // ── Animation graph with both clips ──
    let mut graph = AnimationGraph::new();
    let arming_node = graph.add_clip(animations.add(arming_clip), 1.0, graph.root);
    let intro_node = graph.add_clip(animations.add(intro_clip), 1.0, graph.root);

    // Start intro animation immediately
    let mut anim_player = AnimationPlayer::default();
    anim_player.play(intro_node);

    commands
        .spawn((
            player_name,
            Camera3d::default(),
            Hdr,
            Tonemapping::TonyMcMapface,
            Bloom {
                intensity: cfg.bloom_intensity,
                composite_mode: BloomCompositeMode::Additive,
                ..Bloom::NATURAL
            },
            spawn_transform,
            Player,
            player_target,
            AnimationGraphHandle(graphs.add(graph)),
            anim_player,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    elbow_name,
                    Elbow,
                    Visibility::default(),
                    Transform::from_translation(cfg.pipe_offset).with_rotation(hidden_quat()),
                    elbow_target,
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

    // Observers for animation completion events (global, triggered on AnimationPlayer).
    // Each observer stops the finished clip so the animation system no longer overwrites
    // Transform::rotation in PostUpdate — handing control back to aim_pipe / fly.
    commands.add_observer(
        |_trigger: On<ArmingComplete>,
         mut player_q: Single<&mut AnimationPlayer, With<Player>>,
         node: Res<ArmingAnimNode>,
         mut next_state: ResMut<NextState<crate::GameState>>| {
            player_q.stop(node.0);
            next_state.set(crate::GameState::Running);
        },
    );
    commands.add_observer(
        |_trigger: On<IntroComplete>,
         mut player_q: Single<&mut AnimationPlayer, With<Player>>,
         node: Res<IntroAnimNode>,
         mut next_state: ResMut<NextState<crate::GameState>>| {
            player_q.stop(node.0);
            next_state.set(crate::GameState::Arming);
        },
    );

    // Store animation node indices as resources for the observers / start_arming
    commands.insert_resource(ArmingAnimNode(arming_node));
    commands.insert_resource(IntroAnimNode(intro_node));

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

/// Adds [`AnimatedBy`] to the Elbow entity after it's spawned as a child of Player.
///
/// This runs in a separate startup system because `AnimatedBy` requires the Player entity ID,
/// which isn't available inside `with_children`.
pub fn link_elbow_animation(
    mut commands: Commands,
    player_q: Single<Entity, With<Player>>,
    elbow_q: Single<Entity, With<Elbow>>,
) {
    commands.entity(*player_q).insert(AnimatedBy(*player_q));
    commands.entity(*elbow_q).insert(AnimatedBy(*player_q));
}

/// Resource storing the arming animation node index for triggering on state enter.
#[derive(Resource)]
pub struct ArmingAnimNode(pub AnimationNodeIndex);

/// Resource storing the intro animation node index for stopping on completion.
#[derive(Resource)]
pub struct IntroAnimNode(pub AnimationNodeIndex);

/// Starts the arming animation when entering [`GameState::Arming`].
pub fn start_arming(
    mut player_q: Single<&mut AnimationPlayer, With<Player>>,
    arming_node: Res<ArmingAnimNode>,
) {
    player_q.play(arming_node.0);
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

/// Smoothly rotates the laser pipe toward the aimed hex face, or back to resting angle.
///
/// Uses slerp with an ease-out factor (`aim_speed * dt`) so the pipe decelerates
/// as it approaches the target, giving a physical feel.
pub fn aim_pipe(
    time: Res<Time>,
    cfg: Res<DroneConfig>,
    player_q: Single<&GlobalTransform, With<Player>>,
    mut elbow_q: Single<(&mut Transform, &GlobalTransform), With<Elbow>>,
    sight_target: Query<&GlobalTransform, With<InSight>>,
) {
    let goal = sight_target.single().ok().and_then(|target_gt| {
        let dir = (target_gt.translation() - elbow_q.1.translation()).normalize_or_zero();
        (dir != Vec3::ZERO).then(|| {
            let (_, player_rot, _) = player_q.to_scale_rotation_translation();
            Quat::from_rotation_arc(Vec3::NEG_Y, player_rot.inverse() * dir)
        })
    });
    let goal = goal.unwrap_or_else(armed_quat);

    // Ease-out slerp: large steps when far, small steps near the target
    let t = (cfg.aim_speed * time.delta_secs()).min(1.0);
    let eased = math::ease_out_cubic(t);
    elbow_q.0.rotation = elbow_q.0.rotation.slerp(goal, eased);
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
