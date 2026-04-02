//! Visual test: a quad and a tri mesh with randomized blend materials.
//!
//! Run: `cargo run -p mesh-gradient --example visual --features visual`
//!
//! Every 3 seconds, two new random `StandardMaterial`s are generated and
//! blended via `blend_quad` / `blend_tri`. The left mesh shows the quad
//! gradient, the right mesh shows the tri 3-way blend.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use mesh_gradient::{BlendCfg, blend_quad, blend_tri};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh-gradient visual test".into(),
                resolution: bevy::window::WindowResolution::new(800, 600),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.08)))
        .insert_resource(BlendCfg::default())
        .insert_resource(CycleTimer(Timer::from_seconds(3.0, TimerMode::Repeating)))
        .add_systems(Startup, setup)
        .add_systems(Update, cycle_materials)
        .run();
}

#[derive(Resource)]
struct CycleTimer(Timer);

#[derive(Component)]
struct QuadMesh;

#[derive(Component)]
struct TriMesh;

#[derive(Component)]
struct LabelA;

#[derive(Component)]
struct LabelB;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cfg: Res<BlendCfg>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Lights
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::IDENTITY.looking_to(Vec3::new(-0.3, -1.0, -0.5), Vec3::Y),
    ));
    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
        ..default()
    });

    let (mat_a, mat_b) = random_pair(0);
    let quad_mat = blend_quad(&mat_a, &mat_b, &cfg, &mut images);
    let tri_mat = blend_tri([&mat_a, &mat_b, &mat_a], 0, &cfg, &mut images);

    // Quad mesh (left) — 4-vertex plane
    let quad = meshes.add(mk_quad());
    commands.spawn((
        QuadMesh,
        Mesh3d(quad),
        MeshMaterial3d(materials.add(quad_mat)),
        Transform::from_xyz(-1.5, 0.0, 0.0),
    ));

    // Tri mesh (right) — 3-vertex triangle
    let tri = meshes.add(mk_tri());
    commands.spawn((
        TriMesh,
        Mesh3d(tri),
        MeshMaterial3d(materials.add(tri_mat)),
        Transform::from_xyz(1.5, 0.0, 0.0),
    ));

    // Reference swatches: small planes showing the two raw input materials
    let swatch = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.4)));
    commands.spawn((
        LabelA,
        Mesh3d(swatch.clone()),
        MeshMaterial3d(materials.add(mat_a)),
        Transform::from_xyz(-2.5, 0.0, 1.5),
    ));
    commands.spawn((
        LabelB,
        Mesh3d(swatch),
        MeshMaterial3d(materials.add(mat_b)),
        Transform::from_xyz(-1.5, 0.0, 1.5),
    ));
}

fn cycle_materials(
    time: Res<Time>,
    mut timer: ResMut<CycleTimer>,
    cfg: Res<BlendCfg>,
    mut images: ResMut<Assets<Image>>,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
    mut quads: Query<&mut MeshMaterial3d<StandardMaterial>, With<QuadMesh>>,
    mut tris: Query<&mut MeshMaterial3d<StandardMaterial>, (With<TriMesh>, Without<QuadMesh>)>,
    mut swatch_a: Query<
        &mut MeshMaterial3d<StandardMaterial>,
        (
            With<LabelA>,
            Without<QuadMesh>,
            Without<TriMesh>,
            Without<LabelB>,
        ),
    >,
    mut swatch_b: Query<
        &mut MeshMaterial3d<StandardMaterial>,
        (
            With<LabelB>,
            Without<QuadMesh>,
            Without<TriMesh>,
            Without<LabelA>,
        ),
    >,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let seed = (time.elapsed_secs() * 1000.0) as u32;
    let (mat_a, mat_b) = random_pair(seed);

    let quad_h = mat_assets.add(blend_quad(&mat_a, &mat_b, &cfg, &mut images));
    let tri_h = mat_assets.add(blend_tri([&mat_a, &mat_b, &mat_a], 0, &cfg, &mut images));
    let a_h = mat_assets.add(mat_a);
    let b_h = mat_assets.add(mat_b);

    for mut m in &mut quads {
        m.0 = quad_h.clone();
    }
    for mut m in &mut tris {
        m.0 = tri_h.clone();
    }
    for mut m in &mut swatch_a {
        m.0 = a_h.clone();
    }
    for mut m in &mut swatch_b {
        m.0 = b_h.clone();
    }
}

// ── Helpers ────────────────────────────────────────────────────────

fn random_pair(seed: u32) -> (StandardMaterial, StandardMaterial) {
    let rng = |s: u32| -> f32 {
        let h = s.wrapping_mul(2654435761);
        (h & 0xFFFF) as f32 / 65535.0
    };
    let mk = |s: u32| StandardMaterial {
        base_color: Color::srgb(rng(s), rng(s.wrapping_add(1)), rng(s.wrapping_add(2))),
        perceptual_roughness: 0.3 + rng(s.wrapping_add(3)) * 0.6,
        metallic: rng(s.wrapping_add(4)) * 0.5,
        cull_mode: None,
        ..default()
    };
    (mk(seed), mk(seed.wrapping_add(100)))
}

fn mk_quad() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-1.0, 0.0, -0.5],
            [1.0, 0.0, -0.5],
            [1.0, 0.0, 0.5],
            [-1.0, 0.0, 0.5],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    )
    .with_inserted_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]))
}

fn mk_tri() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[-1.0, 0.0, -0.5], [1.0, 0.0, -0.5], [0.0, 0.0, 0.8]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
    )
    .with_inserted_indices(Indices::U16(vec![0, 1, 2]))
}
