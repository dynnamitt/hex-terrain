use bevy::prelude::*;

/// State machine driving the intro camera animation.
#[derive(Resource)]
pub struct IntroSequence {
    pub(super) phase: IntroPhase,
    pub(super) timer: f32,
    pub(super) start_pitch: Option<f32>,
    pub(super) yaw: Option<f32>,
    /// Whether edge-highlight styling is active (set after tilt-up completes).
    pub highlighting_enabled: bool,
    /// Fires once to trigger the first geometry draw at the origin.
    pub initial_draw_triggered: bool,
    /// `true` once the full intro sequence has finished.
    pub done: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum IntroPhase {
    TiltUp,
    HighlightDelay,
    TiltDown,
    Done,
}

impl IntroSequence {
    pub(super) fn new() -> Self {
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
