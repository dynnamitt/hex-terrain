use bevy::prelude::*;

use super::IntroConfig;
use super::entities::{IntroPhase, IntroTimer};
use crate::drone::Player;
use crate::math;
use crate::{DebugFlag, GameState};

pub fn run_intro(
    time: Res<Time>,
    mut intro: ResMut<IntroTimer>,
    mut query: Query<&mut Transform, With<Player>>,
    intro_cfg: Res<IntroConfig>,
    mut next_state: ResMut<NextState<GameState>>,
    debug: Res<DebugFlag>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    if debug.0 {
        eprintln!(
            "run_intro: phase={:?} timer={:.3} dt={:.3}",
            intro.phase,
            intro.timer,
            time.delta_secs()
        );
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

            let pitch = -intro_cfg.tilt_down_angle.to_radians() * eased;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

            if t >= 1.0 {
                next_state.set(GameState::Running);
            }
        }
    }
}
