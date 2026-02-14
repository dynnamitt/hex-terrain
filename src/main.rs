#![warn(missing_docs)]
//! Hex terrain viewer with neon edge lighting.
//!
//! Renders a hexagonal grid with noise-derived terrain heights, progressive
//! edge/face reveal as the drone moves, and bloom post-processing.

mod drone;
mod intro;
pub mod math;
mod terrain;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
/// Application-wide game state, used for system scheduling.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum GameState {
    /// Intro camera animation playing.
    #[default]
    Intro,
    /// Normal gameplay — drone movement + terrain reveal.
    Running,
    /// Debug overlay active (Tab to toggle).
    Debugging,
}

/// Player world position. Drone/intro write xz + altitude; terrain writes y.
#[derive(Resource, Default, Reflect)]
pub struct PlayerPos {
    /// Final world position (terrain sets `.y`).
    pub pos: Vec3,
    /// User-controlled vertical offset (Q/E/scroll).
    pub altitude: f32,
}

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Hex Terrain".into(),
            ..default()
        }),
        ..default()
    }))
    .register_type::<GameState>()
    .register_type::<PlayerPos>()
    .init_state::<GameState>()
    .init_resource::<PlayerPos>()
    .add_plugins(RemotePlugin::default())
    .add_plugins(RemoteHttpPlugin::default())
    .add_plugins(bevy_egui::EguiPlugin::default())
    .add_plugins(terrain::TerrainPlugin(terrain::TerrainConfig::default()))
    .add_plugins(drone::DronePlugin(drone::DroneConfig::default()))
    .add_plugins(intro::IntroPlugin(intro::IntroConfig::default()))
    .add_systems(Update, exit_on_esc)
    .add_systems(Update, toggle_inspector)
    .add_plugins(WorldInspectorPlugin::new().run_if(in_state(GameState::Debugging)));

    app.run();
}

fn toggle_inspector(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut windows: Query<(&mut CursorOptions, &mut Window)>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        let new_state = match state.get() {
            GameState::Running => GameState::Debugging,
            GameState::Debugging => GameState::Running,
            _ => return,
        };
        let entering_debug = new_state == GameState::Debugging;
        next.set(new_state);
        for (mut opts, mut window) in &mut windows {
            if entering_debug {
                opts.visible = true;
                opts.grab_mode = CursorGrabMode::None;
            } else {
                opts.visible = false;
                opts.grab_mode = CursorGrabMode::Confined;
                let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
                window.set_cursor_position(Some(center));
            }
        }
    }
}

fn exit_on_esc(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
