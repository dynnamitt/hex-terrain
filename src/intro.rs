use bevy::prelude::*;

use crate::camera::{TerrainCamera, interpolate_height};
use crate::grid::{HexGrid, CAMERA_HEIGHT_OFFSET};

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(IntroSequence::new())
            .add_systems(Update, run_intro);
    }
}

#[derive(Resource)]
pub struct IntroSequence {
    phase: IntroPhase,
    timer: f32,
    start_pitch: Option<f32>,
    yaw: Option<f32>,
    pub highlighting_enabled: bool,
    pub initial_draw_triggered: bool,
    pub done: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum IntroPhase {
    TiltUp,
    HighlightDelay,
    TiltDown,
    Done,
}

const TILT_UP_DURATION: f32 = 1.5;
const HIGHLIGHT_DELAY: f32 = 0.7;
const TILT_DOWN_DURATION: f32 = 0.8;

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

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn run_intro(
    time: Res<Time>,
    mut intro: ResMut<IntroSequence>,
    grid: Option<Res<HexGrid>>,
    mut query: Query<&mut Transform, With<TerrainCamera>>,
) {
    if intro.phase == IntroPhase::Done {
        return;
    }

    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    // Interpolate camera height to match terrain during intro
    if let Some(ref grid) = grid {
        let cam_xz = Vec2::new(transform.translation.x, transform.translation.z);
        let target_height = interpolate_height(grid, cam_xz) + CAMERA_HEIGHT_OFFSET;
        transform.translation.y += (target_height - transform.translation.y) * 0.1;
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
            let t = (intro.timer / TILT_UP_DURATION).min(1.0);
            let eased = ease_out_cubic(t);

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
            if intro.timer >= HIGHLIGHT_DELAY {
                intro.phase = IntroPhase::TiltDown;
                intro.timer = 0.0;
            }
        }
        IntroPhase::TiltDown => {
            intro.timer += time.delta_secs();
            let t = (intro.timer / TILT_DOWN_DURATION).min(1.0);
            let eased = ease_out_cubic(t);

            // Tilt down 20 degrees from horizontal
            let pitch = -20.0_f32.to_radians() * eased;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

            if t >= 1.0 {
                intro.phase = IntroPhase::Done;
                intro.done = true;
            }
        }
        IntroPhase::Done => {}
    }
}
