//! Centralized material definitions and FoV material systems for height-based terrain.

use bevy::color::Mix;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::picking::mesh_picking::ray_cast::MeshRayCastSettings;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::HTerrainConfig;
use super::entities::{
    AimStar, FovTransition, HCell, HexFace, InFov, InSight, PreSightMaterial, Quad, QuadEdge, Tri,
};
use super::mineral::{HIGHLIGHT_EMISSIVE, Mineral};
use crate::drone::Player;

/// Base/default terrain color palette (non-mineral items only).
#[derive(Clone, Copy)]
pub enum OrigPalette {
    Debug,
    ClearColor,
}

impl From<OrigPalette> for Color {
    fn from(p: OrigPalette) -> Self {
        match p {
            OrigPalette::Debug => Color::srgb(1.0, 0.2, 0.8),
            OrigPalette::ClearColor => Color::srgb(0.02, 0.03, 0.08),
        }
    }
}

impl From<OrigPalette> for LinearRgba {
    fn from(p: OrigPalette) -> Self {
        match p {
            OrigPalette::Debug => LinearRgba::rgb(4.0, 0.8, 3.2),
            OrigPalette::ClearColor => LinearRgba::BLACK,
        }
    }
}

/// FoV highlight color palette (edge + aim).
#[derive(Clone, Copy)]
pub(super) enum FovPalette {
    Hex,
    Edge,
}

impl From<FovPalette> for Color {
    fn from(p: FovPalette) -> Self {
        match p {
            FovPalette::Hex | FovPalette::Edge => Color::srgb(0.2, 0.9, 0.3),
        }
    }
}

impl From<FovPalette> for LinearRgba {
    fn from(p: FovPalette) -> Self {
        match p {
            FovPalette::Hex | FovPalette::Edge => LinearRgba::rgb(0.04, 0.18, 0.06),
        }
    }
}

/// Material handles for terrain rendering.
///
/// Hex faces use per-[`Mineral`] materials (created at startup, not stored here).
/// Gaps use vertex-colored meshes with a white base material.
/// Edges, aim highlight, and fire effects are stored here.
#[derive(Resource)]
pub struct TerrainMaterials {
    /// Gap base material (white, two-sided — vertex colors provide gradient).
    pub gap_base: Handle<StandardMaterial>,
    /// Gap FoV highlight (tiny emissive boost over vertex colors).
    pub gap_highlight: Handle<StandardMaterial>,
    /// Green emissive material for the aimed-at hex face (screen center + within FoV).
    pub hex_in_aim: Handle<StandardMaterial>,
    /// Aim-star line material (azure glow, slightly more intense than edges).
    pub aim_star: Handle<StandardMaterial>,
    /// Aim-star material while laser is firing (warm yellow glow).
    pub aim_star_firing: Handle<StandardMaterial>,
    /// Hex face material while laser is firing (radial green gradient, no bloom).
    pub hex_during_fire: Handle<StandardMaterial>,
    /// Pre-built aim-star cuboid mesh handle.
    pub aim_star_mesh: Handle<Mesh>,
    /// Bright emissive edge-line material for quad edges.
    pub edge: Handle<StandardMaterial>,
    /// Highlight edge-line material for quad edges within FoV.
    pub edge_highlight: Handle<StandardMaterial>,
}

impl TerrainMaterials {
    pub fn new(
        materials: &mut Assets<StandardMaterial>,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Self {
        Self {
            gap_base: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                cull_mode: None,
                ..default()
            }),
            gap_highlight: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: HIGHLIGHT_EMISSIVE,
                cull_mode: None,
                ..default()
            }),
            hex_in_aim: materials.add(StandardMaterial {
                base_color: FovPalette::Hex.into(),
                ..default()
            }),
            aim_star: materials.add(StandardMaterial {
                base_color: FovPalette::Edge.into(),
                emissive: FovPalette::Edge.into(),
                unlit: true,
                ..default()
            }),
            aim_star_firing: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.0),
                emissive: LinearRgba::new(6.0, 5.0, 0.0, 1.0),
                unlit: true,
                ..default()
            }),
            hex_during_fire: materials.add(StandardMaterial {
                base_color_texture: Some(images.add(radial_gradient(
                    64,
                    [0.4, 1.0, 0.3, 1.0],
                    [0.1, 0.5, 0.15, 1.0],
                    3,
                ))),
                ..default()
            }),
            aim_star_mesh: meshes.add(Cuboid::new(1.6, 0.03, 0.03)),
            edge: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.25, 0.3),
                unlit: true,
                ..default()
            }),
            edge_highlight: materials.add(StandardMaterial {
                base_color: FovPalette::Edge.into(),
                emissive: FovPalette::Edge.into(),
                unlit: true,
                ..default()
            }),
        }
    }

    pub fn debug_material(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        materials.add(StandardMaterial {
            base_color: OrigPalette::Debug.into(),
            emissive: OrigPalette::Debug.into(),
            unlit: true,
            ..default()
        })
    }
}

/// Generates a stepped radial gradient [`Image`] (RGBA, `size`×`size` pixels).
/// `center` color at center, `edge` color at corners, quantized into `steps` concentric bands.
fn radial_gradient(size: u32, center: [f32; 4], edge: [f32; 4], steps: u32) -> Image {
    let half = size as f32 / 2.0;
    let n = (size * size * 4) as usize;
    let mut data = Vec::with_capacity(n);
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 + 0.5 - half) / half;
            let dy = (y as f32 + 0.5 - half) / half;
            let t = (dx * dx + dy * dy).sqrt().min(1.0);
            let t = ((t * steps as f32).floor() / (steps - 1) as f32).min(1.0);
            for i in 0..4 {
                data.push(((center[i] + (edge[i] - center[i]) * t) * 255.0) as u8);
            }
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        min_filter: ImageFilterMode::Linear,
        mag_filter: ImageFilterMode::Linear,
        ..default()
    });
    img
}

/// Bundles InFov change-detection queries and cell→HexFace navigation.
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub(super) struct FovChanges<'w, 's> {
    added_cells: Query<'w, 's, &'static Children, (With<HCell>, Added<InFov>)>,
    added_gaps: Query<'w, 's, Entity, (Or<(With<Quad>, With<Tri>)>, Added<InFov>)>,
    removed: RemovedComponents<'w, 's, InFov>,
    cells: Query<'w, 's, &'static Children, With<HCell>>,
    hex_faces: Query<'w, 's, (), With<HexFace>>,
    in_sight: Query<'w, 's, (), With<InSight>>,
    gap_children: Query<'w, 's, &'static Children, Or<(With<Quad>, With<Tri>)>>,
    quad_edges: Query<'w, 's, (), With<QuadEdge>>,
}

/// Starts or reverses [`FovTransition`] on material entities when [`InFov`] changes.
///
/// Computes per-entity color endpoints based on entity type:
/// - HexFace: mineral color → mineral highlight
/// - Quad/Tri gap: white → white + tiny emissive
/// - QuadEdge: muted cyan → bright green bloom
pub(super) fn start_fov_transitions(
    mut fov: FovChanges,
    mats: Res<TerrainMaterials>,
    minerals: Query<&Mineral>,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut transitions: Query<&mut FovTransition>,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    // Collect (material_entity, fade_in) pairs, then process.
    let mut targets: Vec<(Entity, bool)> = Vec::new();

    for entity in fov.removed.read() {
        if materials.contains(entity) {
            targets.push((entity, false));
            // Propagate to QuadEdge children of removed gap entities.
            if let Ok(children) = fov.gap_children.get(entity) {
                for child in children.iter() {
                    if fov.quad_edges.contains(child) {
                        targets.push((child, false));
                    }
                }
            }
        } else if let Ok(children) = fov.cells.get(entity) {
            for child in children.iter() {
                if fov.hex_faces.contains(child) {
                    targets.push((child, false));
                }
            }
        }
    }
    for children in &fov.added_cells {
        for child in children.iter() {
            if fov.hex_faces.contains(child) {
                targets.push((child, true));
            }
        }
    }
    for entity in &fov.added_gaps {
        targets.push((entity, true));
        // Propagate to QuadEdge children of added gap entities.
        if let Ok(children) = fov.gap_children.get(entity) {
            for child in children.iter() {
                if fov.quad_edges.contains(child) {
                    targets.push((child, true));
                }
            }
        }
    }

    if targets.is_empty() {
        return;
    }

    // Pre-read fixed endpoint colors as LinearRgba to match FovTransition fields.
    let read_ep = |orig: &Handle<StandardMaterial>, hi: &Handle<StandardMaterial>| {
        mat_assets.get(orig).zip(mat_assets.get(hi)).map(|(o, h)| {
            (
                (LinearRgba::from(o.base_color), o.emissive),
                (LinearRgba::from(h.base_color), h.emissive),
            )
        })
    };
    let edge_ep = read_ep(&mats.edge, &mats.edge_highlight);
    let gap_ep = read_ep(&mats.gap_base, &mats.gap_highlight);

    for (entity, fade_in) in targets {
        // InSight entities can't transition — update the stashed target instead.
        if fov.in_sight.contains(entity) {
            if let Ok(&mineral) = minerals.get(entity) {
                let target = if fade_in {
                    mineral.highlight_material()
                } else {
                    mineral.material()
                };
                commands
                    .entity(entity)
                    .insert(PreSightMaterial(mat_assets.add(target)))
                    .remove::<FovTransition>();
            }
            continue;
        }

        let direction = if fade_in { 1.0 } else { -1.0 };
        if let Ok(mut existing) = transitions.get_mut(entity) {
            existing.direction = direction;
        } else {
            let Ok(mut mat) = materials.get_mut(entity) else {
                continue;
            };
            if let Some(current) = mat_assets.get(&mat.0).cloned() {
                mat.0 = mat_assets.add(current);
            }

            let endpoints = if fov.hex_faces.contains(entity) {
                minerals.get(entity).ok().map(|m| {
                    (
                        (LinearRgba::from(m.color()), LinearRgba::BLACK),
                        (LinearRgba::from(m.highlight_color()), HIGHLIGHT_EMISSIVE),
                    )
                })
            } else if fov.quad_edges.contains(entity) {
                edge_ep
            } else {
                gap_ep
            };
            let Some(((orig_base, orig_emissive), (hi_base, hi_emissive))) = endpoints else {
                continue;
            };

            let progress = if fade_in { 0.0 } else { 1.0 };
            commands.entity(entity).insert(FovTransition {
                progress,
                direction,
                orig_base,
                orig_emissive,
                hi_base,
                hi_emissive,
            });
        }
    }
}

/// Ticks [`FovTransition`] progress and lerps material colors each frame.
///
/// Endpoints are stored in the [`FovTransition`] component itself, so this
/// system is mineral-agnostic — no resource lookups needed.
pub(super) fn animate_fov_transitions(
    mut query: Query<
        (
            Entity,
            &mut FovTransition,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<InSight>,
    >,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
    cfg: Res<HTerrainConfig>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let duration = cfg.fov_transition_secs;

    for (entity, mut tr, mat_handle) in &mut query {
        tr.progress = (tr.progress + tr.direction * dt / duration).clamp(0.0, 1.0);
        let t = tr.progress;

        if t <= 0.0 || t >= 1.0 {
            if let Some(mat) = mat_assets.get_mut(&mat_handle.0) {
                let (base, emissive) = if t >= 1.0 {
                    (tr.hi_base, tr.hi_emissive)
                } else {
                    (tr.orig_base, tr.orig_emissive)
                };
                mat.base_color = Color::from(base);
                mat.emissive = emissive;
            }
            commands.entity(entity).remove::<FovTransition>();
        } else if let Some(mat) = mat_assets.get_mut(&mat_handle.0) {
            mat.base_color = Color::from(tr.orig_base.mix(&tr.hi_base, t));
            mat.emissive = tr.orig_emissive.mix(&tr.hi_emissive, t);
        }
    }
}

/// Bundles queries for the [`track_in_sight`] system.
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub(super) struct SightParams<'w, 's> {
    camera: Single<'w, 's, (&'static Camera, &'static GlobalTransform), With<Player>>,
    windows: Single<'w, 's, &'static Window>,
    raycast: MeshRayCast<'w, 's>,
    hex_faces: Query<'w, 's, (), With<HexFace>>,
    current_sight: Query<'w, 's, (Entity, &'static PreSightMaterial), With<InSight>>,
    aim_stars: Query<'w, 's, Entity, With<AimStar>>,
    mats: Res<'w, TerrainMaterials>,
    materials: Query<'w, 's, &'static mut MeshMaterial3d<StandardMaterial>>,
    parents: Query<'w, 's, &'static ChildOf>,
    in_fov: Query<'w, 's, (), With<InFov>>,
}

/// Tags the single hex face at screen center with [`InSight`], applies aim material, and
/// spawns aim-star line children on the targeted face.
///
/// Raycasts first, then compares with the current target — skips all work when the target
/// is unchanged, and performs teardown + apply in a single frame when it changes.
pub(super) fn track_in_sight(mut sight: SightParams, mut commands: Commands) {
    let old_target = sight.current_sight.iter().next().map(|(e, _)| e);
    let new_target = find_aimed_hex_face(&mut sight);

    if old_target == new_target {
        return;
    }

    // Teardown old target (if any)
    if let Some(old) = old_target {
        for entity in &sight.aim_stars {
            commands.entity(entity).despawn();
        }
        if let Ok((_, stashed)) = sight.current_sight.get(old)
            && let Ok(mut mat) = sight.materials.get_mut(old)
        {
            mat.0 = stashed.0.clone();
        }
        commands.entity(old).remove::<(InSight, PreSightMaterial)>();
    }

    // Apply to new target (if any)
    if let Some(new) = new_target {
        if let Ok(mut mat) = sight.materials.get_mut(new) {
            let stash = PreSightMaterial(mat.0.clone());
            mat.0 = sight.mats.hex_in_aim.clone();
            commands
                .entity(new)
                .insert((InSight, stash))
                .remove::<FovTransition>();
        }
        for i in 0..3u32 {
            let angle = i as f32 * std::f32::consts::FRAC_PI_3;
            let child = commands
                .spawn((
                    AimStar,
                    Mesh3d(sight.mats.aim_star_mesh.clone()),
                    MeshMaterial3d(sight.mats.aim_star.clone()),
                    Transform::from_xyz(0.0, 0.01, 0.0).with_rotation(Quat::from_rotation_y(angle)),
                ))
                .id();
            commands.entity(new).add_child(child);
        }
    }
}

/// Raycasts screen center and returns the first in-FoV [`HexFace`] entity hit.
fn find_aimed_hex_face(sight: &mut SightParams) -> Option<Entity> {
    let center = Vec2::new(sight.windows.width() / 2.0, sight.windows.height() / 2.0);
    let (camera, cam_gt) = *sight.camera;
    let ray = camera.viewport_to_world(cam_gt, center).ok()?;
    let filter = |e| sight.hex_faces.contains(e) || sight.aim_stars.contains(e);
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let hits = sight.raycast.cast_ray(ray, &settings);
    for &(entity, _) in hits {
        // Resolve AimStar hits to their parent HexFace.
        let face = if sight.hex_faces.contains(entity) {
            entity
        } else if sight.aim_stars.contains(entity) {
            match sight.parents.get(entity) {
                Ok(parent) if sight.hex_faces.contains(parent.get()) => parent.get(),
                _ => continue,
            }
        } else {
            continue;
        };
        let in_fov = sight
            .parents
            .get(face)
            .ok()
            .is_some_and(|parent| sight.in_fov.contains(parent.get()));
        return in_fov.then_some(face);
    }
    None
}
