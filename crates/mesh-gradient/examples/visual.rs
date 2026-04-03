//! Visual test: quad gradient + tri falloff comparison.
//!
//! Run: `cargo run -p mesh-gradient --example visual --features visual`
//!
//! Every 3 seconds, three random materials cycle. The quad (far left)
//! blends A→B. Two tris compare Hotspot vs Plateau falloff.
//! Sphere indicators at vertices show the raw input materials.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use mesh_gradient::{BlendCfg, TriFalloff, blend_quad, blend_tri};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh-gradient visual test".into(),
                resolution: bevy::window::WindowResolution::new(1100, 500),
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

/// Tri mesh tagged with its falloff mode for updates.
#[derive(Component)]
struct TriTag(TriFalloff);

/// Indicator sphere index (0=A, 1=B, 2=C).
#[derive(Component)]
struct Indicator(u8);

const FALLOFFS: [TriFalloff; 2] = [TriFalloff::Hotspot, TriFalloff::Plateau];

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cfg: Res<BlendCfg>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.5, 5.0).looking_at(Vec3::new(0.0, 0.0, 0.2), Vec3::Y),
    ));
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

    let (a, b, c) = random_tri(0);
    let sphere = meshes.add(Sphere::new(0.12));
    let tri_mesh = meshes.add(mk_tri());

    // ── Quad (far left) ────────────────────────────────────────────
    let qx = -3.5;
    commands.spawn((
        QuadMesh,
        Mesh3d(meshes.add(mk_quad())),
        MeshMaterial3d(materials.add(blend_quad(&a, &b, &cfg, &mut images))),
        Transform::from_xyz(qx, 0.0, 0.0),
    ));
    spawn_ind(
        &mut commands,
        &sphere,
        &mut materials,
        &a,
        Vec3::new(qx - 1.2, 0.0, 0.0),
        0,
    );
    spawn_ind(
        &mut commands,
        &sphere,
        &mut materials,
        &b,
        Vec3::new(qx + 1.2, 0.0, 0.0),
        1,
    );

    // ── Three tris (right of quad) ─────────────────────────────────
    for (i, &falloff) in FALLOFFS.iter().enumerate() {
        let tx = -0.8 + i as f32 * 2.6;
        let mat = blend_tri([&a, &b, &c], 0, falloff, &cfg, &mut images);
        commands.spawn((
            TriTag(falloff),
            Mesh3d(tri_mesh.clone()),
            MeshMaterial3d(materials.add(mat)),
            Transform::from_xyz(tx, 0.0, 0.1),
        ));
        // Vertex indicators (pushed outward from mesh verts)
        spawn_ind(
            &mut commands,
            &sphere,
            &mut materials,
            &a,
            Vec3::new(tx - 1.2, 0.0, -0.7),
            0,
        );
        spawn_ind(
            &mut commands,
            &sphere,
            &mut materials,
            &b,
            Vec3::new(tx + 1.2, 0.0, -0.7),
            1,
        );
        spawn_ind(
            &mut commands,
            &sphere,
            &mut materials,
            &c,
            Vec3::new(tx, 0.0, 1.1),
            2,
        );
    }
}

fn spawn_ind(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    src: &StandardMaterial,
    pos: Vec3,
    idx: u8,
) {
    commands.spawn((
        Indicator(idx),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: src.base_color,
            perceptual_roughness: src.perceptual_roughness,
            metallic: src.metallic,
            ..default()
        })),
        Transform::from_translation(pos),
    ));
}

fn cycle_materials(
    time: Res<Time>,
    mut timer: ResMut<CycleTimer>,
    cfg: Res<BlendCfg>,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut quads: Query<&mut MeshMaterial3d<StandardMaterial>, With<QuadMesh>>,
    mut tris: Query<(&TriTag, &mut MeshMaterial3d<StandardMaterial>), Without<QuadMesh>>,
    mut indicators: Query<
        (&Indicator, &mut MeshMaterial3d<StandardMaterial>),
        (Without<QuadMesh>, Without<TriTag>),
    >,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let seed = (time.elapsed_secs() * 1000.0) as u32;
    let (a, b, c) = random_tri(seed);

    for mut m in &mut quads {
        m.0 = mats.add(blend_quad(&a, &b, &cfg, &mut images));
    }
    for (tag, mut m) in &mut tris {
        m.0 = mats.add(blend_tri([&a, &b, &c], 0, tag.0, &cfg, &mut images));
    }

    let raw = [&a, &b, &c];
    let ind_h: [Handle<StandardMaterial>; 3] = std::array::from_fn(|i| {
        mats.add(StandardMaterial {
            base_color: raw[i].base_color,
            perceptual_roughness: raw[i].perceptual_roughness,
            metallic: raw[i].metallic,
            ..default()
        })
    });
    for (idx, mut m) in &mut indicators {
        m.0 = ind_h[idx.0 as usize].clone();
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Three materials with guaranteed distinct hues (120 apart on the color wheel).
fn random_tri(seed: u32) -> (StandardMaterial, StandardMaterial, StandardMaterial) {
    let rng = |s: u32| -> f32 {
        let h = s.wrapping_mul(2654435761);
        (h & 0xFFFF) as f32 / 65535.0
    };
    let base_hue = rng(seed) * 360.0;
    let hues = [base_hue, base_hue + 120.0, base_hue + 240.0];

    let mk = |i: usize| {
        let s = seed.wrapping_add(i as u32 * 50);
        let sat = 0.5 + rng(s.wrapping_add(10)) * 0.4;
        let val = 0.4 + rng(s.wrapping_add(20)) * 0.5;
        StandardMaterial {
            base_color: Color::hsl(hues[i] % 360.0, sat, val),
            perceptual_roughness: 0.3 + rng(s.wrapping_add(30)) * 0.5,
            metallic: rng(s.wrapping_add(40)) * 0.3,
            cull_mode: None,
            ..default()
        }
    };
    (mk(0), mk(1), mk(2))
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
