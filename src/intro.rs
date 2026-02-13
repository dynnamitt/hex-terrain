//! Intro camera sequence played at startup.
//!
//! Tilts the camera from its initial downward-looking orientation to horizontal,
//! triggers the first geometry draw, then settles into a slight downward angle
//! before handing control to [`crate::camera`].

mod entities;
mod systems;

pub use entities::IntroSequence;

use bevy::prelude::*;

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
            .add_systems(Update, systems::run_intro);
    }
}
