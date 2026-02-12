//! Intro camera sequence played at startup.
//!
//! Tilts the camera from its initial downward-looking orientation to horizontal,
//! triggers the first geometry draw, then settles into a slight downward angle
//! before handing control to [`crate::camera`].

use bevy::prelude::*;

use crate::camera::{CameraConfig, TerrainCamera, interpolate_height};
use crate::grid::HexGrid;
use crate::math;

/// Per-plugin configuration for the intro camera animation.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct IntroConfig {
    /// Duration of the initial tilt-up animation (seconds).
    pub tilt_up_duration: f32,
    /// Pause between tilt-up and tilt-down (seconds).
    pub highlight_delay: f32,
    /// Duration of the settling tilt-down (seconds).
    pub tilt_down_duration: f32,
    /// Downward tilt angle at the end of the intro (degrees).
    pub tilt_down_angle: f32,
}

impl Default for IntroConfig {
    fn default() -> Self {
        Self {
            tilt_up_duration: 1.5,
            highlight_delay: 0.4,
            tilt_down_duration: 0.4,
            tilt_down_angle: 10.0,
        }
    }
}

/// Startup camera animation that tilts from looking down to horizontal.
pub struct IntroPlugin(pub IntroConfig);

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<IntroConfig>()
            .insert_resource(self.0.clone())
            .insert_resource(IntroSequence::new())
            .add_systems(Update, run_intro);
    }
}

/// State machine driving the intro camera animation.
#[derive(Resource)]
pub struct IntroSequence {
    phase: IntroPhase,
    timer: f32,
    start_pitch: Option<f32>,
    yaw: Option<f32>,
    /// Whether edge-highlight styling is active (set after tilt-up completes).
    pub highlighting_enabled: bool,
    /// Fires once to trigger the first geometry draw at the origin.
    pub initial_draw_triggered: bool,
    /// `true` once the full intro sequence has finished.
    pub done: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum IntroPhase {
    TiltUp,
    HighlightDelay,
    TiltDown,
    Done,
}

impl IntroSequence {
    fn new() -> Self {
        Self {
            phase: IntroPhase::TiltUp,
            timer: 0.0,
            start_pitch: None,
            yaw: None,
            highlighting_enabled: false,
            initial_draw_triggered: false,
            done: false,
        }
    }
}

fn run_intro(
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
