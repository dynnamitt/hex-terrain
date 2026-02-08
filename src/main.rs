mod camera;
mod edges;
mod grid;
mod intro;
mod visuals;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::remote::{http::RemoteHttpPlugin, RemotePlugin};
use clap::Parser;

#[derive(Clone, Copy, clap::ValueEnum, Default, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// Only hex perimeter edges (6 edges per hex)
    Perimeter,
    /// Only cross-gap edges (vertex-to-vertex between hexes)
    #[default]
    CrossGap,
    /// Both perimeter + cross-gap (full tessellation)
    Full,
}

#[derive(Clone, Copy, clap::ValueEnum, Default, Debug, PartialEq, Eq)]
pub enum HeightMode {
    /// Smooth: average of parent + neighboring hex center heights
    Smooth,
    /// Blocky: all vertices inherit parent hex center height
    #[default]
    Blocky,
}

#[derive(Parser, Debug)]
#[command(name = "hex-terrain", about = "Hex terrain viewer with neon edges")]
struct Cli {
    /// Render mode for edge display
    #[arg(long, default_value = "cross-gap")]
    mode: RenderMode,

    /// Height mode for vertex computation
    #[arg(long, default_value = "blocky")]
    height_mode: HeightMode,
}

#[derive(Resource, Debug)]
pub struct AppConfig {
    pub render_mode: RenderMode,
    pub height_mode: HeightMode,
}

fn main() {
    let cli = Cli::parse();

    App::new()
        .insert_resource(AppConfig {
            render_mode: cli.mode,
            height_mode: cli.height_mode,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Hex Terrain".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin)
        .add_plugins(visuals::VisualsPlugin)
        .add_plugins(grid::GridPlugin)
        .add_plugins(intro::IntroPlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(edges::EdgesPlugin)
        .add_systems(Update, exit_on_esc)
        .run();
}

fn exit_on_esc(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
