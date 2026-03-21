//! Runtime systems for height-based terrain.

use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastVisibility};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use hexx::{Hex, shapes};

use super::entities::{
    Corner, EmitterMark, HCell, HGrid, HexFace, InFov, InSight, Mark, Quad, QuadEdge, QuadOwner,
    QuadPos1Emitter, QuadPos2Emitter, Tri, TriOwner, TriPos1Emitter, TriPos2Emitter,
};
use super::gaps::GapMeshAccess;
use super::{HTerrainConfig, LaserStrength};
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
            Option<&'static QuadPos1Emitter>,
            Option<&'static QuadPos2Emitter>,
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
        let Ok((qp1, qp2, tp1, tp2)) = lookup.corners.get(corner_entity) else {
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
        out.extend(
            [
                qp1.map(|e| e.0),
                qp2.map(|e| e.0),
                tp1.map(|e| e.0),
                tp2.map(|e| e.0),
            ]
            .into_iter()
            .flatten(),
        );
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

/// Bundles queries for the [`extract_ore`] system.
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub(super) struct OreExtraction<'w, 's> {
    time: Res<'w, Time>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    strength: Res<'w, LaserStrength>,
    sight_face: Query<'w, 's, &'static ChildOf, With<InSight>>,
    cells: Query<'w, 's, &'static mut Transform, (With<HCell>, Without<QuadEdge>)>,
    children: Query<'w, 's, &'static Children>,
    emitters: Query<
        'w,
        's,
        (
            Option<&'static QuadPos1Emitter>,
            Option<&'static QuadPos2Emitter>,
            Option<&'static TriPos1Emitter>,
            Option<&'static TriPos2Emitter>,
        ),
        With<Corner>,
    >,
    owners: Query<'w, 's, (Option<&'static QuadOwner>, Option<&'static TriOwner>), With<Corner>>,
}

/// Lowers an [`HCell`] when the player fires the laser at its [`HexFace`].
///
/// Tick-based: a [`Local`] timer advances only while firing at a target and
/// resets when not. Each tick lowers the cell by [`LaserStrength::extract_height`],
/// then realigns neighboring gap vertices via [`GapMeshAccess`].
pub fn extract_ore(
    mut ore: OreExtraction,
    mut gap_mesh: GapMeshAccess,
    mut timer: Local<Option<Timer>>,
) {
    let firing = ore.keys.pressed(KeyCode::Space) || ore.mouse.pressed(MouseButton::Left);

    if !firing {
        *timer = None;
        return;
    }

    let Some(face_parent) = ore.sight_face.iter().next() else {
        return;
    };
    let cell = face_parent.get();

    let t = timer.get_or_insert_with(|| {
        Timer::from_seconds(ore.strength.extraction_time, TimerMode::Repeating)
    });
    t.tick(ore.time.delta());
    if !t.just_finished() {
        return;
    }

    let Ok(mut tf) = ore.cells.get_mut(cell) else {
        return;
    };
    let extract_h = ore.strength.extract_height;
    tf.translation.y -= extract_h;
    let new_y = tf.translation.y;

    let Ok(cell_children) = ore.children.get(cell) else {
        return;
    };
    for corner in cell_children.iter() {
        // Emitter-side: realign neighbor vertices to new world Y
        if let Ok((qp1, qp2, tp1, tp2)) = ore.emitters.get(corner) {
            for (gap, vi) in [
                qp1.map(|e| (e.gap_entity(), e.vertex_index())),
                qp2.map(|e| (e.gap_entity(), e.vertex_index())),
                tp1.map(|e| (e.gap_entity(), e.vertex_index())),
                tp2.map(|e| (e.gap_entity(), e.vertex_index())),
            ]
            .into_iter()
            .flatten()
            {
                gap_mesh.realign_neighboring_vertex(gap, vi, new_y);
            }
        }
        // Owner-side: shift neighbor vertices up in local space
        if let Ok((qo, to)) = ore.owners.get(corner) {
            for gap in [qo.map(|o| o.gap_entity()), to.map(|o| o.gap_entity())]
                .into_iter()
                .flatten()
            {
                gap_mesh.shift_vertex_y(gap, 1, extract_h);
                gap_mesh.shift_vertex_y(gap, 2, extract_h);
            }
        }
    }
}
