mod camera;
mod edges;
mod grid;
mod visuals;

use bevy::prelude::*;
use bevy::remote::RemotePlugin;
use clap::Parser;

#[derive(Clone, Copy, clap::ValueEnum, Default, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// Only hex perimeter edges (6 edges per hex)
    Perimeter,
    /// Only cross-gap edges (vertex-to-vertex between hexes)
    CrossGap,
    /// Both perimeter + cross-gap (full tessellation)
    #[default]
    Full,
}

#[derive(Clone, Copy, clap::ValueEnum, Default, Debug, PartialEq, Eq)]
pub enum HeightMode {
    /// Smooth: average of parent + neighboring hex center heights
    #[default]
    Smooth,
    /// Blocky: all vertices inherit parent hex center height
    Blocky,
}

#[derive(Parser, Debug)]
#[command(name = "hex-terrain", about = "Hex terrain viewer with neon edges")]
struct Cli {
    /// Render mode for edge display
    #[arg(long, default_value = "full")]
    mode: RenderMode,

    /// Height mode for vertex computation
    #[arg(long, default_value = "smooth")]
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
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin)
        .add_plugins(visuals::VisualsPlugin)
        .add_plugins(grid::GridPlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(edges::EdgesPlugin)
        .run();
}
