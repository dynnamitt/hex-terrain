//! Runtime systems for height-based terrain.

use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use hexx::{Hex, shapes};

use super::HTerrainConfig;
use bevy::color::Mix;

use super::entities::{
    Corner, FovMaterials, FovTransition, HCell, HGrid, HexFace, InFov, InSight, PreSightMaterial,
    Quad, QuadPos2Emitter, QuadPos3Emitter, Tri, TriPos1Emitter, TriPos2Emitter,
};
use crate::drone::Player;
use crate::{GroundLevel, PlayerMoved, PlayerPos};

/// Bundles queries for discovering gap entities (Quad/Tri) reachable from an HCell.
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub(super) struct GapLookup<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    corners: Query<
        'w,
        's,
        (
            Option<&'static QuadPos2Emitter>,
            Option<&'static QuadPos3Emitter>,
            Option<&'static TriPos1Emitter>,
            Option<&'static TriPos2Emitter>,
        ),
        With<Corner>,
    >,
    gaps: Query<'w, 's, (), Or<(With<Quad>, With<Tri>)>>,
}

/// Collects all Quad/Tri entities reachable from an HCell's Corner children.
///
/// Owner path: scans each Corner's children for Quad/Tri meshes.
/// Emitter path: reads `PosXEmitter(Entity)` tuple refs on each Corner.
fn gap_entities_for_cell(cell: Entity, lookup: &GapLookup) -> Vec<Entity> {
    let mut out = Vec::new();
    let Ok(cell_children) = lookup.children.get(cell) else {
        return out;
    };
    for corner_entity in cell_children.iter() {
        let Ok((qp2, qp3, tp1, tp2)) = lookup.corners.get(corner_entity) else {
            continue;
        };
        // Owner path: scan corner's children for gap meshes
        if let Ok(corner_children) = lookup.children.get(corner_entity) {
            for child in corner_children.iter() {
                if lookup.gaps.contains(child) {
                    out.push(child);
                }
            }
        }
        // Emitter path: stored entity refs
        if let Some(e) = qp2 {
            out.push(e.0);
        }
        if let Some(e) = qp3 {
            out.push(e.0);
        }
        if let Some(e) = tp1 {
            out.push(e.0);
        }
        if let Some(e) = tp2 {
            out.push(e.0);
        }
    }
    out
}

/// Sets [`GroundLevel`] from terrain interpolation under the player.
/// Skipped when [`PlayerMoved`] is `false` (no xz/offset change this frame).
pub fn update_ground_level(
    grid: Single<&HGrid>,
    player: Res<PlayerPos>,
    mut moved: ResMut<PlayerMoved>,
    mut ground: ResMut<GroundLevel>,
) {
    if !moved.0 {
        return;
    }
    moved.0 = false;
    ground.0 = Some(grid.terrain.interpolate_height(player.xz));
}

/// Adds/removes [`InFov`] on [`HCell`] entities when the player crosses a hex boundary.
pub fn track_player_fov(
    grid: Single<&HGrid>,
    player: Res<PlayerPos>,
    cfg: Res<HTerrainConfig>,
    mut commands: Commands,
    mut prev_hex: Local<Option<Hex>>,
    gap: GapLookup,
) {
    let current_hex = grid.terrain.world_pos_to_hex(player.xz);

    if *prev_hex == Some(current_hex) {
        return;
    }

    let reach = cfg.grid.fov_reach;
    let new_ring: HashSet<Hex> = shapes::hexagon(current_hex, reach).collect();
    let old_ring: HashSet<Hex> = prev_hex
        .map(|old| shapes::hexagon(old, reach).collect())
        .unwrap_or_default();

    // Remove InFov only from cells that left the FoV
    for hex in old_ring.difference(&new_ring) {
        if let Some(&entity) = grid.hex_entities.get(hex) {
            commands.entity(entity).remove::<InFov>();
        }
    }

    // Add InFov only to cells that newly entered the FoV
    for hex in new_ring.difference(&old_ring) {
        if let Some(&entity) = grid.hex_entities.get(hex) {
            commands.entity(entity).insert(InFov);
        }
    }

    // Diff gap entities separately — gaps are shared between cells, so we must
    // compare the full gap sets rather than piggyback on cell diffs.
    let old_gaps: HashSet<Entity> = old_ring
        .iter()
        .filter_map(|h| grid.hex_entities.get(h).copied())
        .flat_map(|e| gap_entities_for_cell(e, &gap))
        .collect();
    let new_gaps: HashSet<Entity> = new_ring
        .iter()
        .filter_map(|h| grid.hex_entities.get(h).copied())
        .flat_map(|e| gap_entities_for_cell(e, &gap))
        .collect();

    for &entity in old_gaps.difference(&new_gaps) {
        commands.entity(entity).remove::<InFov>();
    }
    for &entity in new_gaps.difference(&old_gaps) {
        commands.entity(entity).insert(InFov);
    }

    *prev_hex = Some(current_hex);
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
}

/// Starts or reverses [`FovTransition`] on material entities when [`InFov`] changes.
pub fn start_fov_transitions(
    mut fov: FovChanges,
    fov_mats: Res<FovMaterials>,
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
    }

    for (entity, fade_in) in targets {
        // InSight entities can't transition — update the stashed target instead.
        if fov.in_sight.contains(entity) {
            let target = if fade_in {
                &fov_mats.hex_highlight
            } else {
                &fov_mats.hex_original
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
pub fn animate_fov_transitions(
    mut query: Query<
        (
            Entity,
            &mut FovTransition,
            &mut MeshMaterial3d<StandardMaterial>,
            Has<HexFace>,
        ),
        Without<InSight>,
    >,
    fov_mats: Res<FovMaterials>,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
    cfg: Res<HTerrainConfig>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let duration = cfg.fov_transition_secs;

    // Copy target colors upfront to avoid borrow conflicts with get_mut below.
    let hex_orig = mat_assets
        .get(&fov_mats.hex_original)
        .map(|m| (m.base_color, m.emissive));
    let hex_hi = mat_assets
        .get(&fov_mats.hex_highlight)
        .map(|m| (m.base_color, m.emissive));
    let gap_orig = mat_assets
        .get(&fov_mats.gap_original)
        .map(|m| (m.base_color, m.emissive));
    let gap_hi = mat_assets
        .get(&fov_mats.gap_highlight)
        .map(|m| (m.base_color, m.emissive));

    let (Some(hex_orig), Some(hex_hi), Some(gap_orig), Some(gap_hi)) =
        (hex_orig, hex_hi, gap_orig, gap_hi)
    else {
        return;
    };

    for (entity, mut tr, mut mat_handle, is_hex) in &mut query {
        tr.progress = (tr.progress + tr.direction * dt / duration).clamp(0.0, 1.0);
        let t = tr.progress;

        let ((orig_base, orig_emissive), (hi_base, hi_emissive)) = if is_hex {
            (hex_orig, hex_hi)
        } else {
            (gap_orig, gap_hi)
        };

        if t <= 0.0 || t >= 1.0 {
            let target = if t >= 1.0 {
                if is_hex {
                    &fov_mats.hex_highlight
                } else {
                    &fov_mats.gap_highlight
                }
            } else if is_hex {
                &fov_mats.hex_original
            } else {
                &fov_mats.gap_original
            };
            mat_handle.0 = target.clone();
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
    fov_mats: Res<'w, FovMaterials>,
    materials: Query<'w, 's, &'static mut MeshMaterial3d<StandardMaterial>>,
    parents: Query<'w, 's, &'static ChildOf>,
    in_fov: Query<'w, 's, (), With<InFov>>,
}

/// Tags the single hex face at screen center with [`InSight`] and applies a purple material.
pub fn track_in_sight(mut sight: SightParams, mut commands: Commands) {
    // Remove previous InSight — restore pre-sight material
    for (entity, stashed) in &sight.current_sight {
        commands
            .entity(entity)
            .remove::<(InSight, PreSightMaterial)>();
        if let Ok(mut mat) = sight.materials.get_mut(entity) {
            mat.0 = stashed.0.clone();
        }
    }

    // Ray from screen center
    let center = Vec2::new(sight.windows.width() / 2.0, sight.windows.height() / 2.0);
    let (camera, cam_gt) = *sight.camera;
    let Ok(ray) = camera.viewport_to_world(cam_gt, center) else {
        return;
    };

    // Cast and find first HexFace hit
    let hits = sight.raycast.cast_ray(ray, &default());
    for &(entity, _) in hits {
        if sight.hex_faces.contains(entity) {
            // Only highlight if parent HCell is within FoV
            let in_fov = sight
                .parents
                .get(entity)
                .ok()
                .is_some_and(|parent| sight.in_fov.contains(parent.get()));
            if !in_fov {
                return;
            }
            if let Ok(mut mat) = sight.materials.get_mut(entity) {
                let stash = PreSightMaterial(mat.0.clone());
                mat.0 = sight.fov_mats.hex_in_aim.clone();
                commands.entity(entity).insert((InSight, stash));
            }
            return;
        }
    }
}
