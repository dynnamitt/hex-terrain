//! Centralized material definitions and FoV material systems for height-based terrain.

use bevy::color::Mix;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::HTerrainConfig;
use super::entities::{
    AimStar, FovTransition, HCell, HexFace, InFov, InSight, PreSightMaterial, Quad, QuadEdge, Tri,
};
use crate::drone::Player;

/// Base/default terrain color palette.
#[derive(Clone, Copy)]
pub enum OrigPalette {
    Hex,
    Gap,
    Edge,
    Debug,
    ClearColor,
}

impl From<OrigPalette> for Color {
    fn from(p: OrigPalette) -> Self {
        match p {
            OrigPalette::Hex => Color::srgb(0.4, 0.5, 0.1), // olive green
            OrigPalette::Gap => Color::srgb(0.1, 0.1, 0.04), // near-black brown
            OrigPalette::Edge => Color::srgb(0.0, 0.5, 1.0), // azure blue
            OrigPalette::Debug => Color::srgb(1.0, 0.2, 0.8), // hot pink
            OrigPalette::ClearColor => Color::srgb(0.01, 0.01, 0.02), // near-black navy
        }
    }
}

impl From<OrigPalette> for LinearRgba {
    fn from(p: OrigPalette) -> Self {
        match p {
            OrigPalette::Gap => LinearRgba::rgb(0.0255, 0.051, 0.085), // dim navy glow
            OrigPalette::Edge => LinearRgba::rgb(0.0, 20.0, 40.0),     // intense cyan bloom
            OrigPalette::Debug => LinearRgba::rgb(4.0, 0.8, 3.2),      // bright magenta bloom
            _ => LinearRgba::BLACK,
        }
    }
}

/// FoV highlight + aim color palette.
#[derive(Clone, Copy)]
pub enum FovPalette {
    Hex,
    Gap,
    Edge,
    Aim,
}

impl From<FovPalette> for Color {
    fn from(p: FovPalette) -> Self {
        match p {
            FovPalette::Hex | FovPalette::Edge => Color::srgb(0.2, 0.9, 0.3), // bright green
            FovPalette::Gap => Color::srgb(0.165, 0.745, 0.25),               // muted lime green
            FovPalette::Aim => Color::srgb(0.6, 0.1, 0.8),                    // purple
        }
    }
}

impl From<FovPalette> for LinearRgba {
    fn from(p: FovPalette) -> Self {
        match p {
            FovPalette::Hex | FovPalette::Edge => LinearRgba::rgb(0.04, 0.18, 0.06), // green-tinted glow
            FovPalette::Gap => LinearRgba::rgb(0.022, 0.10, 0.034), // soft green glow
            FovPalette::Aim => LinearRgba::rgb(0.92, 0.32, 2.56),   // vivid violet bloom
        }
    }
}

/// Material handles for terrain rendering: hex faces, gaps, aim highlight, and edges.
#[derive(Resource)]
pub struct TerrainMaterials {
    /// Original hex face material.
    pub hex_original: Handle<StandardMaterial>,
    /// Highlight hex face material (emissive warm glow).
    pub hex_highlight: Handle<StandardMaterial>,
    /// Original gap (Quad/Tri) material.
    pub gap_original: Handle<StandardMaterial>,
    /// Highlight gap material (emissive cyan glow).
    pub gap_highlight: Handle<StandardMaterial>,
    /// Green emissive material for the aimed-at hex face (screen center + within FoV).
    pub hex_in_aim: Handle<StandardMaterial>,
    /// Aim-star line material (azure glow, slightly more intense than edges).
    pub aim_star: Handle<StandardMaterial>,
    /// Pre-built aim-star cuboid mesh handle.
    pub aim_star_mesh: Handle<Mesh>,
    /// Bright emissive edge-line material for quad edges.
    pub edge: Handle<StandardMaterial>,
    /// Highlight edge-line material for quad edges within FoV.
    pub edge_highlight: Handle<StandardMaterial>,
}

impl TerrainMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>, meshes: &mut Assets<Mesh>) -> Self {
        Self {
            hex_original: materials.add(StandardMaterial {
                base_color: OrigPalette::Hex.into(),
                ..default()
            }),
            hex_highlight: materials.add(StandardMaterial {
                base_color: FovPalette::Hex.into(),
                emissive: FovPalette::Hex.into(),
                ..default()
            }),
            gap_original: materials.add(StandardMaterial {
                base_color: OrigPalette::Gap.into(),
                emissive: OrigPalette::Gap.into(),
                cull_mode: None,
                ..default()
            }),
            gap_highlight: materials.add(StandardMaterial {
                base_color: FovPalette::Gap.into(),
                emissive: FovPalette::Gap.into(),
                cull_mode: None,
                ..default()
            }),
            hex_in_aim: materials.add(StandardMaterial {
                base_color: FovPalette::Hex.into(),
                emissive: LinearRgba::rgb(0.06, 0.27, 0.09),
                ..default()
            }),
            aim_star: materials.add(StandardMaterial {
                base_color: FovPalette::Edge.into(),
                emissive: FovPalette::Edge.into(),
                unlit: true,
                ..default()
            }),
            aim_star_mesh: meshes.add(Cuboid::new(1.6, 0.03, 0.03)),
            edge: materials.add(StandardMaterial {
                base_color: OrigPalette::Edge.into(),
                emissive: OrigPalette::Edge.into(),
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
pub(super) fn start_fov_transitions(
    mut fov: FovChanges,
    mats: Res<TerrainMaterials>,
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

    for (entity, fade_in) in targets {
        // InSight entities can't transition — update the stashed target instead.
        if fov.in_sight.contains(entity) {
            let target = if fade_in {
                &mats.hex_highlight
            } else {
                &mats.hex_original
            };
            commands
                .entity(entity)
                .insert(PreSightMaterial(target.clone()))
                .remove::<FovTransition>();
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
            let progress = if fade_in { 0.0 } else { 1.0 };
            commands.entity(entity).insert(FovTransition {
                progress,
                direction,
            });
        }
    }
}

/// Ticks [`FovTransition`] progress and lerps material colors each frame.
#[allow(clippy::type_complexity)]
pub(super) fn animate_fov_transitions(
    mut query: Query<
        (
            Entity,
            &mut FovTransition,
            &mut MeshMaterial3d<StandardMaterial>,
            Has<HexFace>,
            Has<QuadEdge>,
        ),
        Without<InSight>,
    >,
    mats: Res<TerrainMaterials>,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
    cfg: Res<HTerrainConfig>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let duration = cfg.fov_transition_secs;

    // Copy target colors upfront to avoid borrow conflicts with get_mut below.
    let hex_orig = mat_assets
        .get(&mats.hex_original)
        .map(|m| (m.base_color, m.emissive));
    let hex_hi = mat_assets
        .get(&mats.hex_highlight)
        .map(|m| (m.base_color, m.emissive));
    let gap_orig = mat_assets
        .get(&mats.gap_original)
        .map(|m| (m.base_color, m.emissive));
    let gap_hi = mat_assets
        .get(&mats.gap_highlight)
        .map(|m| (m.base_color, m.emissive));
    let edge_orig = mat_assets
        .get(&mats.edge)
        .map(|m| (m.base_color, m.emissive));
    let edge_hi = mat_assets
        .get(&mats.edge_highlight)
        .map(|m| (m.base_color, m.emissive));

    let (
        Some(hex_orig),
        Some(hex_hi),
        Some(gap_orig),
        Some(gap_hi),
        Some(edge_orig),
        Some(edge_hi),
    ) = (hex_orig, hex_hi, gap_orig, gap_hi, edge_orig, edge_hi)
    else {
        return;
    };

    for (entity, mut tr, mat_handle, is_hex, is_edge) in &mut query {
        tr.progress = (tr.progress + tr.direction * dt / duration).clamp(0.0, 1.0);
        let t = tr.progress;

        let ((orig_base, orig_emissive), (hi_base, hi_emissive)) = if is_hex {
            (hex_orig, hex_hi)
        } else if is_edge {
            (edge_orig, edge_hi)
        } else {
            (gap_orig, gap_hi)
        };

        if t <= 0.0 || t >= 1.0 {
            if let Some(mat) = mat_assets.get_mut(&mat_handle.0) {
                let (base, emissive) = if t >= 1.0 {
                    (hi_base, hi_emissive)
                } else {
                    (orig_base, orig_emissive)
                };
                mat.base_color = base;
                mat.emissive = emissive;
            }
            commands.entity(entity).remove::<FovTransition>();
        } else if let Some(mat) = mat_assets.get_mut(&mat_handle.0) {
            let orig_lin = LinearRgba::from(orig_base);
            let hi_lin = LinearRgba::from(hi_base);
            mat.base_color = Color::from(orig_lin.mix(&hi_lin, t));
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
    let hits = sight.raycast.cast_ray(ray, &default());
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
