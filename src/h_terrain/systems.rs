//! Runtime systems for height-based terrain.

use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastVisibility};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use hexx::{Hex, shapes};

use super::HTerrainConfig;

use super::entities::{
    Corner, HGrid, HexFace, InFov, Quad, QuadPos2Emitter, QuadPos3Emitter, Tri, TriPos1Emitter,
    TriPos2Emitter,
};
use crate::{GroundLevel, PlayerPos};

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

/// Sets [`GroundLevel`] by raycasting straight down onto terrain meshes.
/// On miss (e.g. grid edge), keeps the previous height unchanged.
#[allow(clippy::type_complexity)]
pub fn update_ground_level(
    player: Res<PlayerPos>,
    mut ground: ResMut<GroundLevel>,
    mut raycast: MeshRayCast,
    surfaces: Query<(), Or<(With<HexFace>, With<Quad>, With<Tri>)>>,
) {
    let origin_y = ground.0.unwrap_or(0.0) + 100.0;
    let origin = Vec3::new(player.xz.x, origin_y, player.xz.y);
    let ray = Ray3d::new(origin, Dir3::NEG_Y);
    let filter = |e| surfaces.contains(e);
    let settings = MeshRayCastSettings::default()
        .with_filter(&filter)
        .with_visibility(RayCastVisibility::Any);

    if let Some((_, hit)) = raycast.cast_ray(ray, &settings).first() {
        ground.0 = Some(hit.point.y);
    }
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
