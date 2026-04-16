//! Centralized material definitions and FoV material systems for height-based terrain.

use bevy::color::Mix;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::MeshRayCastSettings;
use bevy::prelude::*;

use super::HTerrainConfig;
use super::entities::{
    BaseMaterial, FovTransition, HCell, HexFace, InFov, InSight, Quad, QuadEdge, Tri,
};
use super::fov_overlay::{BubbleFovMaterial, BubbleFovOverlay, FovMaterial, FovOverlay};
use crate::drone::Player;

/// Edge highlight color (sRGB) — cyan.
const EDGE_COLOR: Color = Color::srgb(0.2, 0.9, 0.9);
/// Edge highlight emissive (linear) — cyan.
const EDGE_EMISSIVE: LinearRgba = LinearRgba::rgb(0.04, 0.18, 0.18);

/// Material handles for terrain rendering.
///
/// Hex faces and gaps use per-entity [`FovMaterial`] (created at startup, not stored here).
/// Edges are stored here; aim-star is shader-drawn via `FovOverlay`.
#[derive(Resource)]
pub struct TerrainMaterials {
    /// Muted edge-line material for quad edges outside FoV.
    pub edge: Handle<StandardMaterial>,
    /// Highlight edge-line material for quad edges within FoV.
    pub edge_highlight: Handle<StandardMaterial>,
}

impl TerrainMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            edge: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.25, 0.3),
                unlit: true,
                ..default()
            }),
            edge_highlight: materials.add(StandardMaterial {
                base_color: EDGE_COLOR,
                emissive: EDGE_EMISSIVE,
                unlit: true,
                ..default()
            }),
        }
    }

    pub fn debug_material(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.2, 0.8),
            emissive: LinearRgba::rgb(4.0, 0.8, 3.2),
            unlit: true,
            ..default()
        })
    }
}

/// Bundles InFov change-detection queries and cell→face/edge navigation.
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub(super) struct FovChanges<'w, 's> {
    added_cells: Query<'w, 's, &'static Children, (With<HCell>, Added<InFov>)>,
    added_gaps: Query<'w, 's, Entity, (Or<(With<Quad>, With<Tri>)>, Added<InFov>)>,
    removed: RemovedComponents<'w, 's, InFov>,
    cells: Query<'w, 's, &'static Children, With<HCell>>,
    hex_faces: Query<'w, 's, (), With<HexFace>>,
    /// All gap entities (Quad has Children, Tri does not).
    gaps: Query<'w, 's, (), Or<(With<Quad>, With<Tri>)>>,
    gap_children: Query<'w, 's, &'static Children, Or<(With<Quad>, With<Tri>)>>,
    quad_edges: Query<'w, 's, (), With<QuadEdge>>,
}

/// Starts or reverses [`FovTransition`] on face and edge entities when [`InFov`] changes.
///
/// Face entities (HexFace, Quad, Tri) are swapped from `StandardMaterial` to
/// `FovMaterial` on FoV entry so the shader only runs on the visible ring.
/// Edge entities (`QuadEdge`) keep the existing endpoint-based color lerp.
#[allow(clippy::too_many_arguments)]
pub(super) fn start_fov_transitions(
    mut fov: FovChanges,
    mut transitions: Query<&mut FovTransition>,
    base_mats: Query<&BaseMaterial>,
    quads: Query<(), With<Quad>>,
    mut edge_materials: Query<&mut MeshMaterial3d<StandardMaterial>, With<QuadEdge>>,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
    mut fov_assets: ResMut<Assets<FovMaterial>>,
    mut bubble_assets: ResMut<Assets<BubbleFovMaterial>>,
    mut commands: Commands,
) {
    let mut targets: Vec<(Entity, bool)> = Vec::new();

    // Collect removed entities
    for entity in fov.removed.read() {
        if fov.hex_faces.contains(entity) || fov.gaps.contains(entity) {
            targets.push((entity, false));
            // QuadEdge children of removed gap
            if let Ok(children) = fov.gap_children.get(entity) {
                for child in children.iter() {
                    if fov.quad_edges.contains(child) {
                        targets.push((child, false));
                    }
                }
            }
        } else if let Ok(children) = fov.cells.get(entity) {
            // HCell removed — propagate to HexFace child
            for child in children.iter() {
                if fov.hex_faces.contains(child) {
                    targets.push((child, false));
                }
            }
        }
    }

    // Collect added HexFace children of added HCells
    for children in &fov.added_cells {
        for child in children.iter() {
            if fov.hex_faces.contains(child) {
                targets.push((child, true));
            }
        }
    }

    // Collect added gap entities + their QuadEdge children
    for entity in &fov.added_gaps {
        targets.push((entity, true));
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

    for (entity, fade_in) in targets {
        let direction = if fade_in { 1.0 } else { -1.0 };

        // Reverse existing transition if present
        if let Ok(mut existing) = transitions.get_mut(entity) {
            existing.direction = direction;
            continue;
        }

        // QuadEdge: clone shared material so animation doesn't affect all edges
        if let Ok(mut mat) = edge_materials.get_mut(entity)
            && let Some(current) = mat_assets.get(&mat.0).cloned()
        {
            mat.0 = mat_assets.add(current);
        }

        // Face entities entering FoV: swap StandardMaterial → shader material
        if fade_in && let Ok(base) = base_mats.get(entity) {
            let std_mat = mat_assets.get(&base.0).cloned().unwrap_or_default();
            if fov.hex_faces.contains(entity) {
                // HexFace → FovMaterial (aiming_overlay.wgsl)
                let h = fov_assets.add(FovMaterial {
                    base: std_mat,
                    extension: FovOverlay {
                        data: Vec4::new(0.0, 0.0, 0.0, 0.0),
                        aim_params: Vec4::ZERO,
                    },
                });
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .insert(MeshMaterial3d(h));
            } else {
                // Quad/Tri → BubbleFovMaterial (fov_bubbles.wgsl)
                let shape_type = if quads.contains(entity) { 1.0 } else { 2.0 };
                let h = bubble_assets.add(BubbleFovMaterial {
                    base: std_mat,
                    extension: BubbleFovOverlay {
                        data: Vec4::new(0.0, 0.0, shape_type, 0.0),
                        aim_params: Vec4::ZERO,
                    },
                });
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .insert(MeshMaterial3d(h));
            }
        }

        let progress = if fade_in { 0.0 } else { 1.0 };
        commands.entity(entity).insert(FovTransition {
            progress,
            direction,
        });
    }
}

/// Animates [`FovTransition`] on face entities by updating the `FovOverlay` uniform.
///
/// When a fade-out completes (progress ≤ 0), swaps back to `StandardMaterial`
/// from [`BaseMaterial`] so the custom shader no longer runs on that face.
pub(super) fn animate_face_fov(
    mut query: Query<
        (
            Entity,
            &mut FovTransition,
            &MeshMaterial3d<FovMaterial>,
            &BaseMaterial,
        ),
        Without<QuadEdge>,
    >,
    mut fov_assets: ResMut<Assets<FovMaterial>>,
    cfg: Res<HTerrainConfig>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let duration = cfg.fov_transition_secs;

    for (entity, mut tr, mat_handle, base) in &mut query {
        tr.progress = (tr.progress + tr.direction * dt / duration).clamp(0.0, 1.0);

        if let Some(mat) = fov_assets.get_mut(&mat_handle.0) {
            mat.extension.data.x = tr.progress;
        }

        if tr.progress <= 0.0 || tr.progress >= 1.0 {
            commands.entity(entity).remove::<FovTransition>();
            // Fade-out complete: swap FovMaterial → StandardMaterial
            if tr.progress <= 0.0 {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<FovMaterial>>()
                    .insert(MeshMaterial3d(base.0.clone()));
            }
        }
    }
}

/// Animates [`FovTransition`] on gap face entities by updating [`BubbleFovOverlay`] progress.
///
/// Mirrors [`animate_face_fov`] but for gap faces using [`BubbleFovMaterial`].
pub(super) fn animate_gap_fov(
    mut query: Query<
        (
            Entity,
            &mut FovTransition,
            &MeshMaterial3d<BubbleFovMaterial>,
            &BaseMaterial,
        ),
        Without<QuadEdge>,
    >,
    mut bubble_assets: ResMut<Assets<BubbleFovMaterial>>,
    cfg: Res<HTerrainConfig>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let duration = cfg.fov_transition_secs;

    for (entity, mut tr, mat_handle, base) in &mut query {
        tr.progress = (tr.progress + tr.direction * dt / duration).clamp(0.0, 1.0);

        if let Some(mat) = bubble_assets.get_mut(&mat_handle.0) {
            mat.extension.data.x = tr.progress;
        }

        if tr.progress <= 0.0 || tr.progress >= 1.0 {
            commands.entity(entity).remove::<FovTransition>();
            if tr.progress <= 0.0 {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<BubbleFovMaterial>>()
                    .insert(MeshMaterial3d(base.0.clone()));
            }
        }
    }
}

/// Animates [`FovTransition`] on edge entities by lerping `StandardMaterial` colors.
pub(super) fn animate_edge_fov(
    mut query: Query<
        (
            Entity,
            &mut FovTransition,
            &MeshMaterial3d<StandardMaterial>,
        ),
        With<QuadEdge>,
    >,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
    mats: Res<TerrainMaterials>,
    cfg: Res<HTerrainConfig>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let duration = cfg.fov_transition_secs;

    // Read edge endpoints once
    let Some(((orig_base, orig_emissive), (hi_base, hi_emissive))) = mat_assets
        .get(&mats.edge)
        .zip(mat_assets.get(&mats.edge_highlight))
        .map(|(o, h)| {
            (
                (LinearRgba::from(o.base_color), o.emissive),
                (LinearRgba::from(h.base_color), h.emissive),
            )
        })
    else {
        return;
    };

    for (entity, mut tr, mat_handle) in &mut query {
        tr.progress = (tr.progress + tr.direction * dt / duration).clamp(0.0, 1.0);
        let t = tr.progress;

        if t <= 0.0 || t >= 1.0 {
            if let Some(mat) = mat_assets.get_mut(&mat_handle.0) {
                let (base, emissive) = if t >= 1.0 {
                    (hi_base, hi_emissive)
                } else {
                    (orig_base, orig_emissive)
                };
                mat.base_color = Color::from(base);
                mat.emissive = emissive;
            }
            commands.entity(entity).remove::<FovTransition>();
        } else if let Some(mat) = mat_assets.get_mut(&mat_handle.0) {
            mat.base_color = Color::from(orig_base.mix(&hi_base, t));
            mat.emissive = orig_emissive.mix(&hi_emissive, t);
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
    current_sight: Query<'w, 's, Entity, With<InSight>>,
    cfg: Res<'w, HTerrainConfig>,
    face_mats: Query<'w, 's, &'static MeshMaterial3d<FovMaterial>>,
    fov_assets: ResMut<'w, Assets<FovMaterial>>,
    parents: Query<'w, 's, &'static ChildOf>,
    in_fov: Query<'w, 's, (), With<InFov>>,
}

/// Tags the single hex face at screen center with [`InSight`] and sets
/// `aim_mode` + `aim_star_rotate_pace` uniforms on its [`FovMaterial`].
pub(super) fn track_in_sight(mut sight: SightParams, mut commands: Commands) {
    let old_target = sight.current_sight.iter().next();
    let new_target = find_aimed_hex_face(&mut sight);

    if old_target == new_target {
        return;
    }

    // Teardown old target
    if let Some(old) = old_target {
        if let Ok(mat_handle) = sight.face_mats.get(old)
            && let Some(mat) = sight.fov_assets.get_mut(&mat_handle.0)
        {
            mat.extension.data.y = 0.0;
            mat.extension.data.w = 0.0;
            mat.extension.aim_params = Vec4::ZERO;
        }
        commands.entity(old).remove::<InSight>();
    }

    // Apply to new target
    if let Some(new) = new_target {
        let cfg = &sight.cfg;
        if let Ok(mat_handle) = sight.face_mats.get(new)
            && let Some(mat) = sight.fov_assets.get_mut(&mat_handle.0)
        {
            mat.extension.data.y = 1.0;
            mat.extension.data.w = cfg.aim_star_rotate_pace;
            mat.extension.aim_params = Vec4::new(
                cfg.aim_star_radius,
                cfg.aim_star_inner_cut,
                cfg.aim_star_thickness,
                0.0,
            );
        }
        commands.entity(new).insert(InSight);
    }
}

/// Raycasts screen center and returns the first in-FoV [`HexFace`] entity hit.
fn find_aimed_hex_face(sight: &mut SightParams) -> Option<Entity> {
    let center = Vec2::new(sight.windows.width() / 2.0, sight.windows.height() / 2.0);
    let (camera, cam_gt) = *sight.camera;
    let ray = camera.viewport_to_world(cam_gt, center).ok()?;
    let filter = |e| sight.hex_faces.contains(e);
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let hits = sight.raycast.cast_ray(ray, &settings);
    let &(face, _) = hits.first()?;
    sight
        .parents
        .get(face)
        .ok()
        .is_some_and(|parent| sight.in_fov.contains(parent.get()))
        .then_some(face)
}
