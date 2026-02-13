use bevy::prelude::*;

use super::IntroConfig;
use super::entities::{IntroPhase, IntroSequence};
use crate::camera::{CameraConfig, TerrainCamera, interpolate_height};
use crate::grid::HexGrid;
use crate::math;

pub fn run_intro(
    time: Res<Time>,
    mut intro: ResMut<IntroSequence>,
    grid_q: Query<&HexGrid>,
    mut query: Query<&mut Transform, With<TerrainCamera>>,
    intro_cfg: Res<IntroConfig>,
    cam_cfg: Res<CameraConfig>,
) {
    if intro.phase == IntroPhase::Done {
        return;
    }

    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    // Interpolate camera height to match terrain during intro
    if let Ok(grid) = grid_q.single() {
        let cam_xz = Vec2::new(transform.translation.x, transform.translation.z);
        let target_height = interpolate_height(grid, cam_xz) + cam_cfg.height_offset;
        transform.translation.y += (target_height - transform.translation.y) * cam_cfg.height_lerp;
    }

    // Capture initial orientation on first frame
    if intro.start_pitch.is_none() {
        let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        intro.start_pitch = Some(pitch);
        intro.yaw = Some(yaw);
    }

    let start_pitch = intro.start_pitch.unwrap();
    let yaw = intro.yaw.unwrap();

    match intro.phase {
        IntroPhase::TiltUp => {
            intro.timer += time.delta_secs();
            let t = (intro.timer / intro_cfg.tilt_up_duration).min(1.0);
            let eased = math::ease_out_cubic(t);

            // Interpolate pitch from start (looking down) to 0 (horizontal)
            let pitch = start_pitch * (1.0 - eased);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

            if t >= 1.0 {
                intro.phase = IntroPhase::HighlightDelay;
                intro.timer = 0.0;
                intro.highlighting_enabled = true;
                intro.initial_draw_triggered = true;
            }
        }
        IntroPhase::HighlightDelay => {
            intro.timer += time.delta_secs();
            if intro.timer >= intro_cfg.highlight_delay {
                intro.phase = IntroPhase::TiltDown;
                intro.timer = 0.0;
            }
        }
        IntroPhase::TiltDown => {
            intro.timer += time.delta_secs();
            let t = (intro.timer / intro_cfg.tilt_down_duration).min(1.0);
            let eased = math::ease_out_cubic(t);

            // Tilt down by configured angle from horizontal
            let pitch = -intro_cfg.tilt_down_angle.to_radians() * eased;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

            if t >= 1.0 {
                intro.phase = IntroPhase::Done;
                intro.done = true;
            }
        }
        IntroPhase::Done => {}
    }
}
