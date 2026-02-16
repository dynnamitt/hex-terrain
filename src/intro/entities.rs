use bevy::prelude::*;

/// Timing state for the intro camera animation.
#[derive(Resource)]
pub struct IntroTimer {
    pub(super) phase: IntroPhase,
    pub(super) timer: f32,
    pub(super) start_pitch: Option<f32>,
    pub(super) yaw: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum IntroPhase {
    TiltUp,
    HighlightDelay,
    TiltDown,
}

impl IntroTimer {
    pub(super) fn new() -> Self {
        Self {
            phase: IntroPhase::TiltUp,
            timer: 0.0,
            start_pitch: None,
            yaw: None,
        }
    }
}
