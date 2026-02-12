//! First-person camera controller and hex-cell tracking.
//!
//! WASD + mouse look with terrain-height interpolation. [`CameraCell`] reports
//! which hex the camera currently occupies; downstream systems in `edges` use
//! its change flag to spawn geometry and restyle visited cells.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use hexx::Hex;

use bevy::window::{CursorGrabMode, CursorOptions, WindowFocused};

use crate::InspectorActive;
use crate::grid::HexGrid;
use crate::intro::IntroSequence;
use crate::math;
use crate::petals::HexEntities;

/// Per-plugin configuration for the camera controller.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct CameraConfig {
    /// WASD movement speed in world-units per second.
    pub move_speed: f32,
    /// Horizontal mouse sensitivity (radians per pixel).
    pub mouse_sensitivity_x: f32,
    /// Vertical mouse sensitivity (radians per pixel).
    pub mouse_sensitivity_y: f32,
    /// Pixel margin from window edge that triggers cursor recentering.
    pub edge_margin: f32,
    /// Margin from vertical to prevent camera flip (radians).
    pub pitch_margin: f32,
    /// Lerp factor for smooth height transitions per frame.
    pub height_lerp: f32,
    /// Vertical offset of the camera above the terrain surface.
    pub height_offset: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            move_speed: 15.0,
            mouse_sensitivity_x: 0.003,
            mouse_sensitivity_y: 0.002,
            edge_margin: 100.0,
            pitch_margin: 0.05,
            height_lerp: 0.1,
            height_offset: 16.0,
        }
    }
}

/// First-person camera controller with WASD movement, mouse look, and terrain following.
pub struct CameraPlugin(pub CameraConfig);

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TerrainCamera>()
            .register_type::<CameraConfig>()
            .insert_resource(self.0.clone())
            .init_resource::<CameraCell>()
            .init_resource::<CursorRecentered>()
            .add_systems(Startup, hide_cursor)
            .add_systems(
                Update,
                recenter_cursor.run_if(|active: Res<InspectorActive>| !active.0),
            )
            .add_systems(
                Update,
                (move_camera, track_camera_cell)
                    .chain()
                    .after(recenter_cursor)
                    .run_if(|intro: Res<IntroSequence>| intro.done)
                    .run_if(|active: Res<InspectorActive>| !active.0),
            );
    }
}

/// Marker component for the player-controlled camera entity.
#[derive(Component, Reflect)]
pub struct TerrainCamera;

/// Tracks which hex cell the camera currently occupies.
#[derive(Resource, Default)]
pub struct CameraCell {
    /// Hex coordinate directly below the camera.
    pub current: Hex,
    /// The cell the camera occupied last frame (if it moved).
    pub previous: Option<Hex>,
    /// `true` for exactly one frame after a cell transition.
    pub changed: bool,
}

/// Set to `true` on frames where the cursor was warped back to center,
/// so [`move_camera`] can discard any synthetic mouse-motion delta.
#[derive(Resource, Default)]
struct CursorRecentered(bool);

#[allow(clippy::too_many_arguments)]
fn move_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    grid_q: Query<&HexGrid>,
    mut query: Query<&mut Transform, With<TerrainCamera>>,
    recentered: Res<CursorRecentered>,
    cfg: Res<CameraConfig>,
) {
    let Ok(grid) = grid_q.single() else { return };
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    // Mouse look: yaw (horizontal) + pitch (vertical)
    // Skip deltas on frames where cursor was warped to avoid camera jerk.
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
        let delta = direction * cfg.move_speed * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.z += delta.z;
    }

    // Height interpolation from nearest vertices
    let cam_xz = Vec2::new(transform.translation.x, transform.translation.z);
    let target_height = interpolate_height(grid, cam_xz) + cfg.height_offset;
    // Smooth height transition
    transform.translation.y += (target_height - transform.translation.y) * cfg.height_lerp;
}

/// Inverse-distance-weighted height interpolation from nearby hex vertices.
///
/// Samples the six vertices of the hex under `pos` plus all neighbouring
/// hexes, then blends their heights by `1/dist²`. Returns the hex centre
/// height as a fallback when no vertices are in range.
pub fn interpolate_height(grid: &HexGrid, pos: Vec2) -> f32 {
    // Find nearest vertices by distance and inverse-distance weight
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    // Find the hex under the camera
    let hex = grid.layout.world_pos_to_hex(pos);

    // Check this hex and its neighbors for nearby vertices
    let hexes_to_check: Vec<Hex> = std::iter::once(hex).chain(hex.all_neighbors()).collect();

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

/// Updates [`CameraCell`] when the camera crosses into a new hex.
pub fn track_camera_cell(
    grid_q: Query<&HexGrid>,
    hex_entities: Option<Res<HexEntities>>,
    names: Query<&Name>,
    mut cell: ResMut<CameraCell>,
    query: Query<&Transform, With<TerrainCamera>>,
) {
    let Ok(grid) = grid_q.single() else { return };
    let Ok(transform) = query.single() else {
        return;
    };

    let pos = Vec2::new(transform.translation.x, transform.translation.z);
    let new_hex = grid.layout.world_pos_to_hex(pos);

    let first_frame = cell.previous.is_none();
    if new_hex != cell.current || first_frame {
        cell.previous = Some(cell.current);
        cell.current = new_hex;
        cell.changed = true;

        if let Some(name) = hex_entities
            .as_ref()
            .and_then(|he| he.map.get(&new_hex))
            .and_then(|&e| names.get(e).ok())
        {
            #[cfg(debug_assertions)]
            println!("Camera over: {name}");
        }
    } else {
        cell.changed = false;
    }
}

fn hide_cursor(mut q: Query<(&mut CursorOptions, &mut Window)>) {
    for (mut opts, mut window) in &mut q {
        opts.visible = false;
        opts.grab_mode = CursorGrabMode::Confined;
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
    }
}

/// Warps cursor back to center when it drifts near a window edge or when
/// the window regains focus. Sets [`CursorRecentered`] so the camera system
/// can discard any synthetic mouse-motion that frame.
fn recenter_cursor(
    mut windows: Query<&mut Window>,
    mut focus_events: MessageReader<WindowFocused>,
    mut recentered: ResMut<CursorRecentered>,
    cfg: Res<CameraConfig>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::platform::collections::HashMap;
    use hexx::HexLayout;

    /// Builds a minimal [`HexGrid`] with a single hex at the origin whose
    /// six vertices all sit at the given height.
    fn single_hex_grid(height: f32) -> HexGrid {
        let layout = HexLayout {
            scale: Vec2::splat(4.0),
            ..default()
        };
        let unit_layout = HexLayout {
            scale: Vec2::splat(1.0),
            ..default()
        };
        let hex = Hex::ZERO;
        let center_2d = layout.hex_to_world_pos(hex);
        let corners = unit_layout.center_aligned_hex_corners();
        let radius = 1.0;

        let mut vertex_positions = HashMap::new();
        for (i, corner) in corners.iter().enumerate() {
            let offset = *corner * radius;
            let world_x = center_2d.x + offset.x;
            let world_z = center_2d.y + offset.y;
            vertex_positions.insert((hex, i as u8), Vec3::new(world_x, height, world_z));
        }

        let mut heights = HashMap::new();
        heights.insert(hex, height);

        HexGrid {
            layout,
            heights,
            radii: HashMap::new(),
            vertex_positions,
        }
    }

    #[test]
    fn interpolate_at_vertex_returns_vertex_height() {
        let grid = single_hex_grid(5.0);
        // Query a position very close to a vertex
        let vpos = grid.vertex_positions[&(Hex::ZERO, 0)];
        let pos = Vec2::new(vpos.x + 0.0001, vpos.z + 0.0001);
        let h = interpolate_height(&grid, pos);
        assert!(
            (h - 5.0).abs() < 0.1,
            "height near vertex should be ~5.0, got {h}"
        );
    }

    #[test]
    fn interpolate_at_center_returns_vertex_height_when_uniform() {
        let grid = single_hex_grid(3.0);
        // All vertices are at 3.0, so any IDW blend should also give 3.0
        let h = interpolate_height(&grid, Vec2::ZERO);
        assert!(
            (h - 3.0).abs() < 0.1,
            "uniform height should be ~3.0, got {h}"
        );
    }

    #[test]
    fn interpolate_outside_grid_falls_back() {
        let grid = single_hex_grid(7.0);
        // Far away from any vertex — should fallback to hex center height or 0
        let h = interpolate_height(&grid, Vec2::new(1000.0, 1000.0));
        // Falls back to grid.heights or 0.0
        assert!(h >= 0.0);
    }
}
