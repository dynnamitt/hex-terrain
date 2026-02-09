#![warn(missing_docs)]
//! Hex terrain viewer with neon edge lighting.
//!
//! Renders a hexagonal grid with noise-derived terrain heights, progressive
//! edge/face reveal as the camera moves, and bloom post-processing. CLI flags
//! select the [`RenderMode`].

mod camera;
mod edges;
mod grid;
mod intro;
pub mod math;
mod visuals;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::Parser;

/// Which edge categories to render on the hex grid.
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

#[derive(Parser, Debug)]
#[command(name = "hex-terrain", about = "Hex terrain viewer with neon edges")]
struct Cli {
    /// Render mode for edge display
    #[arg(long, default_value = "full")]
    mode: RenderMode,
}

/// Top-level configuration derived from CLI arguments.
#[derive(Resource, Debug)]
pub struct AppConfig {
    /// Which edge categories are drawn each frame.
    pub render_mode: RenderMode,
}

/// Whether the `bevy-inspector-egui` world inspector overlay is visible.
#[derive(Resource, Default)]
pub struct InspectorActive(
    /// `true` while the inspector panel is shown.
    pub bool,
);

fn main() {
    let cli = Cli::parse();

    App::new()
        .insert_resource(AppConfig {
            render_mode: cli.mode,
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
        .add_plugins(WorldInspectorPlugin::new().run_if(|active: Res<InspectorActive>| active.0))
        .add_plugins(visuals::VisualsPlugin)
        .add_plugins(grid::GridPlugin)
        .add_plugins(intro::IntroPlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(edges::EdgesPlugin)
        .init_resource::<InspectorActive>()
        .add_systems(Update, toggle_inspector)
        .add_systems(Update, exit_on_esc)
        .run();
}

fn toggle_inspector(
    keys: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<InspectorActive>,
    mut windows: Query<(&mut CursorOptions, &mut Window)>,
) {
    if keys.just_pressed(KeyCode::KeyI) {
        active.0 = !active.0;
        for (mut opts, mut window) in &mut windows {
            if active.0 {
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
