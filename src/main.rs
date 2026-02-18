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
use bevy_egui::egui;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::Parser;

/// Hex terrain viewer with neon edge lighting.
#[derive(Parser)]
struct Cli {
    /// Start in debug mode (GameState::Inspecting).
    #[arg(long)]
    debug: bool,

    /// Override intro tilt-up duration (seconds).
    #[arg(long)]
    intro_duration: Option<f32>,
}
/// Application-wide game state, used for system scheduling.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum GameState {
    /// Intro camera animation playing.
    #[default]
    Intro,
    /// Normal gameplay — drone movement + terrain reveal.
    Running,
    /// Inspector overlay active (Tab to toggle).
    Inspecting,
}

/// CLI debug flag exposed as a resource for verbose logging.
#[derive(Resource)]
pub struct DebugFlag(pub bool);

/// Player world position. Drone/intro write xz + altitude; terrain writes y.
#[derive(Resource, Default, Reflect)]
pub struct PlayerPos {
    /// Final world position (terrain sets `.y`).
    pub pos: Vec3,
    /// User-controlled vertical offset (Q/E/scroll).
    pub altitude: f32,
}

fn main() {
    let cli = Cli::parse();

    let mut intro_cfg = intro::IntroConfig::default();
    if let Some(d) = cli.intro_duration {
        intro_cfg.tilt_up_duration = d;
    }
    if cli.debug {
        eprintln!(
            "IntroConfig: tilt_up_duration={}",
            intro_cfg.tilt_up_duration
        );
    }

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
    .insert_resource(DebugFlag(cli.debug))
    .add_plugins(RemotePlugin::default())
    .add_plugins(RemoteHttpPlugin::default())
    .add_plugins(bevy_egui::EguiPlugin::default())
    .add_plugins(terrain::TerrainPlugin(terrain::TerrainConfig::default()))
    .add_plugins(drone::DronePlugin(drone::DroneConfig::default()))
    .add_plugins(intro::IntroPlugin(intro_cfg))
    .add_systems(Update, exit_on_esc)
    .add_systems(Update, toggle_inspector)
    .add_systems(Update, draw_fps.run_if(|f: Res<DebugFlag>| f.0))
    .add_plugins(WorldInspectorPlugin::new().run_if(in_state(GameState::Inspecting)));

    app.run();
}

fn draw_fps(
    mut egui_ctx: Query<&mut bevy_egui::EguiContext>,
    time: Res<Time>,
    mut ready: Local<bool>,
) {
    // Skip first frame — bevy_egui hasn't called Context::run() yet.
    if !*ready {
        *ready = true;
        return;
    }
    let Ok(mut ctx) = egui_ctx.single_mut() else {
        return;
    };
    let fps = 1.0 / time.delta_secs().max(f32::EPSILON);
    egui::Area::new(egui::Id::new("fps_overlay"))
        .fixed_pos(egui::pos2(8.0, 8.0))
        .show(ctx.get_mut(), |ui| {
            ui.label(
                egui::RichText::new(format!("{fps:.0} fps"))
                    .color(egui::Color32::from_rgb(0, 255, 128))
                    .font(egui::FontId::monospace(14.0)),
            );
        });
}

fn toggle_inspector(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut windows: Query<(&mut CursorOptions, &mut Window)>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        let new_state = match state.get() {
            GameState::Running => GameState::Inspecting,
            GameState::Inspecting => GameState::Running,
            _ => return,
        };
        let entering_inspect = new_state == GameState::Inspecting;
        next.set(new_state);
        for (mut opts, mut window) in &mut windows {
            if entering_inspect {
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
