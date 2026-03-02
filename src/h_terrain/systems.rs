//! Runtime systems for height-based terrain.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use hexx::{Hex, shapes};

use super::HTerrainConfig;
use super::entities::{
    Corner, FovMaterials, HCell, HGrid, HexFace, InFov, Quad, QuadPos2Emitter, QuadPos3Emitter,
    Tri, TriPos1Emitter, TriPos2Emitter,
};
use crate::{PlayerMoved, PlayerPos};

/// Bundles queries for discovering gap entities (Quad/Tri) reachable from an HCell.
#[derive(SystemParam)]
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

/// Sets `PlayerPos.pos.y` from terrain interpolation.
/// Skipped when [`PlayerMoved`] is `false` (no xz/altitude change this frame).
pub fn update_player_height(
    grid: Single<&HGrid>,
    mut player: ResMut<PlayerPos>,
    mut moved: ResMut<PlayerMoved>,
) {
    if !moved.0 {
        return;
    }
    moved.0 = false;
    let xz = Vec2::new(player.pos.x, player.pos.z);
    player.pos.y = grid.terrain.interpolate_height(xz) + player.altitude;
}

/// Seeds [`PlayerPos`] from the camera transform left by the intro sequence.
///
/// Sets xz position, derives altitude from terrain height, and computes the
/// initial `pos.y` so the first `fly()` frame doesn't snap to the origin.
pub fn sync_initial_altitude(
    grid: Single<&HGrid>,
    mut player: ResMut<PlayerPos>,
    mut moved: ResMut<PlayerMoved>,
    cam_tf: Single<&Transform, With<crate::drone::Player>>,
) {
    let xz = Vec2::new(cam_tf.translation.x, cam_tf.translation.z);
    let terrain_h = grid.terrain.interpolate_height(xz);
    player.pos.x = cam_tf.translation.x;
    player.pos.z = cam_tf.translation.z;
    player.altitude = cam_tf.translation.y - terrain_h;
    player.pos.y = cam_tf.translation.y;
    moved.0 = true;
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
    let xz = Vec2::new(player.pos.x, player.pos.z);
    let current_hex = grid.terrain.world_pos_to_hex(xz);

    if *prev_hex == Some(current_hex) {
        return;
    }

    let reach = cfg.grid.fov_reach;

    // Remove InFov from old ring
    if let Some(old) = *prev_hex {
        for hex in shapes::hexagon(old, reach) {
            if let Some(&entity) = grid.hex_entities.get(&hex) {
                commands.entity(entity).remove::<InFov>();
                for gap_entity in gap_entities_for_cell(entity, &gap) {
                    commands.entity(gap_entity).remove::<InFov>();
                }
            }
        }
    }

    // Add InFov to new ring
    for hex in shapes::hexagon(current_hex, reach) {
        if let Some(&entity) = grid.hex_entities.get(&hex) {
            commands.entity(entity).insert(InFov);
            for gap_entity in gap_entities_for_cell(entity, &gap) {
                commands.entity(gap_entity).insert(InFov);
            }
        }
    }

    *prev_hex = Some(current_hex);
}

/// Swaps materials on hex/quad/tri meshes when [`InFov`] is added or removed.
pub fn highlight_fov(
    added_cells: Query<&Children, (With<HCell>, Added<InFov>)>,
    added_gaps: Query<Entity, (Or<(With<Quad>, With<Tri>)>, Added<InFov>)>,
    mut removed: RemovedComponents<InFov>,
    cells: Query<&Children, With<HCell>>,
    hex_faces: Query<(), With<HexFace>>,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    fov_mats: Res<FovMaterials>,
) {
    // Removals: swap back to original material
    for entity in removed.read() {
        // Quad/Tri have MeshMaterial3d directly
        if let Ok(mut mat) = materials.get_mut(entity) {
            mat.0 = fov_mats.gap_original.clone();
        } else if let Ok(children) = cells.get(entity) {
            // HCell → find HexFace child
            for child in children.iter() {
                if hex_faces.contains(child) {
                    if let Ok(mut mat) = materials.get_mut(child) {
                        mat.0 = fov_mats.hex_original.clone();
                    }
                }
            }
        }
    }

    // Additions: swap to highlight material
    for children in &added_cells {
        for child in children.iter() {
            if hex_faces.contains(child) {
                if let Ok(mut mat) = materials.get_mut(child) {
                    mat.0 = fov_mats.hex_highlight.clone();
                }
            }
        }
    }
    for entity in &added_gaps {
        if let Ok(mut mat) = materials.get_mut(entity) {
            mat.0 = fov_mats.gap_highlight.clone();
        }
    }
}
