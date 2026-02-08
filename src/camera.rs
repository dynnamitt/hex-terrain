use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use hexx::Hex;

use bevy::window::{CursorGrabMode, CursorOptions};

use crate::grid::{HexGrid, CAMERA_HEIGHT_OFFSET};
use crate::intro::IntroSequence;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraCell>()
            .add_systems(Startup, hide_cursor)
            .add_systems(
                Update,
                (move_camera, track_camera_cell)
                    .chain()
                    .run_if(|intro: Res<IntroSequence>| intro.done),
            );
    }
}

#[derive(Component)]
pub struct TerrainCamera;

#[derive(Resource, Default)]
pub struct CameraCell {
    pub current: Hex,
    pub previous: Option<Hex>,
    pub changed: bool,
}

const MOVE_SPEED: f32 = 15.0;
const MOUSE_SENSITIVITY: f32 = 0.003;

fn move_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    grid: Option<Res<HexGrid>>,
    mut query: Query<&mut Transform, With<TerrainCamera>>,
) {
    let Some(grid) = grid else { return };
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    // Mouse look: yaw (horizontal) + pitch (vertical)
    let mut yaw = 0.0;
    let mut pitch = 0.0;
    for ev in mouse_motion.read() {
        yaw -= ev.delta.x * MOUSE_SENSITIVITY;
        pitch -= ev.delta.y * MOUSE_SENSITIVITY;
    }
    if yaw != 0.0 {
        transform.rotate_y(yaw);
    }
    if pitch != 0.0 {
        // Apply pitch on local X axis, clamped to avoid flipping
        let (_, current_pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let clamped_pitch =
            (current_pitch + pitch).clamp(-std::f32::consts::FRAC_PI_2 + 0.05, std::f32::consts::FRAC_PI_2 - 0.05);
        let pitch_delta = clamped_pitch - current_pitch;
        transform.rotate_local_x(pitch_delta);
    }

    // WASD movement in the camera's forward/right plane (XZ only)
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
        let delta = direction * MOVE_SPEED * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.z += delta.z;
    }

    // Height interpolation from nearest vertices
    let cam_xz = Vec2::new(transform.translation.x, transform.translation.z);
    let target_height = interpolate_height(&grid, cam_xz) + CAMERA_HEIGHT_OFFSET;
    // Smooth height transition
    transform.translation.y += (target_height - transform.translation.y) * 0.1;
}

pub fn interpolate_height(grid: &HexGrid, pos: Vec2) -> f32 {
    // Find nearest vertices by distance and inverse-distance weight
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    // Find the hex under the camera
    let hex = grid.layout.world_pos_to_hex(pos);

    // Check this hex and its neighbors for nearby vertices
    let hexes_to_check: Vec<Hex> = std::iter::once(hex)
        .chain(hex.all_neighbors())
        .collect();

    for h in hexes_to_check {
        for i in 0..6u8 {
            if let Some(&vpos) = grid.vertex_positions.get(&(h, i)) {
                let dx = pos.x - vpos.x;
                let dz = pos.y - vpos.z;
                let dist_sq = dx * dx + dz * dz;
                if dist_sq < 0.001 {
                    return vpos.y;
                }
                let weight = 1.0 / dist_sq;
                weighted_sum += vpos.y * weight;
                weight_total += weight;
            }
        }
    }

    if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        // Fallback to hex center height
        grid.heights.get(&hex).copied().unwrap_or(0.0)
    }
}

pub fn track_camera_cell(
    grid: Option<Res<HexGrid>>,
    mut cell: ResMut<CameraCell>,
    query: Query<&Transform, With<TerrainCamera>>,
) {
    let Some(grid) = grid else { return };
    let Ok(transform) = query.single() else {
        return;
    };

    let pos = Vec2::new(transform.translation.x, transform.translation.z);
    let new_hex = grid.layout.world_pos_to_hex(pos);

    if new_hex != cell.current {
        cell.previous = Some(cell.current);
        cell.current = new_hex;
        cell.changed = true;
    } else {
        cell.changed = false;
    }
}

fn hide_cursor(mut cursor_q: Query<&mut CursorOptions>) {
    for mut opts in &mut cursor_q {
        opts.visible = false;
        opts.grab_mode = CursorGrabMode::Locked;
    }
}

