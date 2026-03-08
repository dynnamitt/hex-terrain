//! ECS integration tests for h_terrain startup and runtime systems.

use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use hexx::{Hex, shapes};

use super::entities::{Corner, FovTransition, HCell, HGrid, HexFace, InFov, Quad, QuadEdge, Tri};
use super::materials::TerrainMaterials;
use super::{HTerrainConfig, HTerrainPhase, materials, math, startup_systems, systems};
use crate::{DebugFlag, GameState, GroundLevel, PlayerMoved, PlayerPos};

fn test_config() -> HTerrainConfig {
    HTerrainConfig {
        grid: super::HGridSettings {
            radius: 2,
            fov_reach: 1,
            point_spacing: 4.0,
            height_noise_seed: 43,
            radius_noise_seed: 137,
            height_noise_octaves: 4,
            radius_noise_octaves: 3,
            height_noise_scale: 50.0,
            radius_noise_scale: 30.0,
            max_height: 20.0,
            min_hex_radius: 0.2,
            max_hex_radius: 2.6,
        },
        clear_color: Color::BLACK,
        fov_transition_secs: 0.3,
    }
}

fn test_app() -> App {
    test_app_with_config(test_config())
}

fn test_app_with_config(cfg: HTerrainConfig) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(StatesPlugin)
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(cfg.clone())
        .insert_resource(DebugFlag(false))
        .init_resource::<PlayerPos>()
        .init_resource::<GroundLevel>()
        .init_resource::<PlayerMoved>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )))
        .init_state::<GameState>();

    // Force state to Running immediately.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Running);

    app.configure_sets(
        Update,
        (
            HTerrainPhase::UpdateGround,
            HTerrainPhase::TrackFov.after(HTerrainPhase::UpdateGround),
            HTerrainPhase::Highlight.after(HTerrainPhase::TrackFov),
        ),
    );

    app.add_systems(
        Startup,
        startup_systems::seed_ground_level.after(startup_systems::generate_h_grid),
    );
    app.add_systems(Startup, startup_systems::generate_h_grid);

    // Register Update systems (omit track_in_sight — requires camera/window).
    app.add_systems(
        Update,
        (
            systems::update_ground_level.in_set(HTerrainPhase::UpdateGround),
            systems::track_player_fov.in_set(HTerrainPhase::TrackFov),
            materials::start_fov_transitions.in_set(HTerrainPhase::Highlight),
            materials::animate_fov_transitions.after(HTerrainPhase::Highlight),
        )
            .run_if(in_state(GameState::Running)),
    );

    // First update: runs Startup + one Update frame.
    app.update();
    // State transition needs another frame to apply.
    app.update();

    app
}

fn move_player(app: &mut App, xz: Vec2) {
    app.world_mut().resource_mut::<PlayerPos>().xz = xz;
    app.world_mut().resource_mut::<PlayerMoved>().0 = true;
}

// ── Startup verification ────────────────────────────────────────

#[test]
fn startup_spawns_grid_entities() {
    let mut app = test_app();
    let w = app.world_mut();

    let grid_count = w.query::<&HGrid>().iter(w).count();
    assert_eq!(grid_count, 1, "should have exactly 1 HGrid");

    let cell_count = w.query::<&HCell>().iter(w).count();
    assert_eq!(cell_count, 19, "radius-2 grid should have 19 HCells");

    let face_count = w.query::<&HexFace>().iter(w).count();
    assert_eq!(face_count, 19, "should have 19 HexFace entities");

    let corner_count = w.query::<&Corner>().iter(w).count();
    assert_eq!(corner_count, 19 * 6, "should have 114 Corner entities");

    let hex_entity_count = w
        .query::<&HGrid>()
        .iter(w)
        .next()
        .unwrap()
        .hex_entities
        .len();
    assert_eq!(hex_entity_count, 19);

    assert!(
        w.get_resource::<TerrainMaterials>().is_some(),
        "TerrainMaterials resource should exist"
    );
}

#[test]
fn startup_spawns_gap_entities() {
    let mut app = test_app();

    let hexes: Vec<Hex> = shapes::hexagon(Hex::ZERO, 2).collect();
    let (expected_quads, expected_tris) = math::gap_filler(&hexes);

    let w = app.world_mut();
    let quad_count = w.query::<&Quad>().iter(w).count();
    let tri_count = w.query::<&Tri>().iter(w).count();
    assert_eq!(
        quad_count, expected_quads,
        "Quad count should match gap_filler"
    );
    assert_eq!(
        tri_count, expected_tris,
        "Tri count should match gap_filler"
    );

    let edge_count = w.query::<&QuadEdge>().iter(w).count();
    assert_eq!(
        edge_count,
        4 * expected_quads,
        "QuadEdge count should be 4 × quads"
    );
}

#[test]
fn seed_ground_level_sets_height() {
    let mut app = test_app();

    let ground_val = app.world().resource::<GroundLevel>().0;
    assert!(ground_val.is_some(), "GroundLevel should be seeded");

    let w = app.world_mut();
    let expected = w
        .query::<&HGrid>()
        .iter(w)
        .next()
        .unwrap()
        .terrain
        .interpolate_height(Vec2::ZERO);
    let actual = ground_val.unwrap();
    assert!(
        (actual - expected).abs() < 1e-4,
        "GroundLevel {actual} should match interpolated height {expected}"
    );
}

// ── update_ground_level ────────────────────────────────────────

#[test]
fn ground_level_updates_on_player_move() {
    let mut app = test_app();

    move_player(&mut app, Vec2::new(8.0, 0.0));
    app.update();

    let actual = app.world().resource::<GroundLevel>().0.unwrap();
    let moved = app.world().resource::<PlayerMoved>().0;

    let w = app.world_mut();
    let expected = w
        .query::<&HGrid>()
        .iter(w)
        .next()
        .unwrap()
        .terrain
        .interpolate_height(Vec2::new(8.0, 0.0));

    assert!(
        (actual - expected).abs() < 1e-4,
        "GroundLevel {actual} should match interpolated height {expected} after move"
    );
    assert!(!moved, "PlayerMoved should be consumed");
}

#[test]
fn ground_level_skips_when_not_moved() {
    let mut app = test_app();

    app.world_mut().resource_mut::<PlayerMoved>().0 = false;
    app.world_mut().resource_mut::<GroundLevel>().0 = None;
    app.update();

    assert!(
        app.world().resource::<GroundLevel>().0.is_none(),
        "GroundLevel should stay None when PlayerMoved is false"
    );
}

// ── track_player_fov ────────────────────────────────────────────

#[test]
fn initial_fov_marks_center_ring() {
    let mut app = test_app();
    let w = app.world_mut();

    let in_fov_count = w.query_filtered::<&HCell, With<InFov>>().iter(w).count();

    // Player at origin, fov_reach=1 → origin + 6 neighbors = 7
    assert_eq!(
        in_fov_count, 7,
        "InFov HCell count should be 7 at origin with fov_reach=1"
    );
}

#[test]
fn fov_follows_player_across_hex_boundary() {
    let mut cfg = test_config();
    cfg.fov_transition_secs = 10.0; // slow so transitions stay visible
    let mut app = test_app_with_config(cfg);

    // Collect cells that have InFov before the move.
    let before_fov: Vec<Entity> = {
        let w = app.world_mut();
        w.query_filtered::<Entity, (With<HCell>, With<InFov>)>()
            .iter(w)
            .collect()
    };
    assert_eq!(before_fov.len(), 7);

    // Complete any initial transitions so we start clean.
    for _ in 0..5 {
        app.update();
    }

    let target_pos = {
        let w = app.world_mut();
        w.query::<&HGrid>()
            .iter(w)
            .next()
            .unwrap()
            .terrain
            .hex_to_world_pos(Hex::new(1, 0))
    };

    move_player(&mut app, target_pos);
    app.update();

    let w = app.world_mut();
    let in_fov_count = w.query_filtered::<&HCell, With<InFov>>().iter(w).count();
    assert_eq!(in_fov_count, 7, "InFov count should be 7 at hex(1,0)");

    // Cells that were InFov before AND after the move must NOT have a
    // FovTransition — their InFov was never removed, so no animation should fire.
    let after_fov: Vec<Entity> = w
        .query_filtered::<Entity, (With<HCell>, With<InFov>)>()
        .iter(w)
        .collect();
    let overlap: Vec<Entity> = before_fov
        .iter()
        .filter(|e| after_fov.contains(e))
        .copied()
        .collect();
    assert!(
        !overlap.is_empty(),
        "There should be overlap cells between old and new FoV"
    );

    // Check that overlap HexFace children have no FovTransition.
    for &cell in &overlap {
        let children = w.entity(cell).get::<Children>().unwrap();
        for child in children.iter() {
            if w.entity(child).contains::<HexFace>() {
                assert!(
                    !w.entity(child).contains::<FovTransition>(),
                    "Overlap cell's HexFace should NOT have FovTransition (glitch)"
                );
            }
        }
    }
}

#[test]
fn fov_gap_counts_stable_across_hex_move() {
    // Use radius=4, fov_reach=2 so both origin and hex(1,0) are fully interior
    // (2-hex buffer between FoV edge and grid edge) — gap counts are symmetric.
    let mut cfg = test_config();
    cfg.grid.radius = 4;
    cfg.grid.fov_reach = 2;
    let mut app = test_app_with_config(cfg);

    // Phase A: baseline at origin (fov_reach=2 → 19 cells)
    let w = app.world_mut();
    let cell_count_a = w
        .query_filtered::<(), (With<HCell>, With<InFov>)>()
        .iter(w)
        .count();
    let quad_count_a = w
        .query_filtered::<(), (With<Quad>, With<InFov>)>()
        .iter(w)
        .count();
    let tri_count_a = w
        .query_filtered::<(), (With<Tri>, With<InFov>)>()
        .iter(w)
        .count();
    assert_eq!(cell_count_a, 19);
    let total_a = cell_count_a + quad_count_a + tri_count_a;

    // Phase B: move to hex(1,0) — fully interior with radius=4
    let target_pos = w
        .query::<&HGrid>()
        .iter(w)
        .next()
        .unwrap()
        .terrain
        .hex_to_world_pos(Hex::new(1, 0));

    move_player(&mut app, target_pos);
    app.update();

    let w = app.world_mut();
    let cell_count_b = w
        .query_filtered::<(), (With<HCell>, With<InFov>)>()
        .iter(w)
        .count();
    let quad_count_b = w
        .query_filtered::<(), (With<Quad>, With<InFov>)>()
        .iter(w)
        .count();
    let tri_count_b = w
        .query_filtered::<(), (With<Tri>, With<InFov>)>()
        .iter(w)
        .count();
    assert_eq!(cell_count_b, 19, "hex(1,0) should have 19 InFov cells");
    let total_b = cell_count_b + quad_count_b + tri_count_b;

    assert_eq!(
        total_b, total_a,
        "InFov geo total should be stable: phase A had {total_a} (c={cell_count_a} q={quad_count_a} t={tri_count_a}), \
         phase B has {total_b} (c={cell_count_b} q={quad_count_b} t={tri_count_b})"
    );
}

#[test]
fn fov_at_grid_edge_partial() {
    let mut app = test_app();

    let edge_pos = {
        let w = app.world_mut();
        w.query::<&HGrid>()
            .iter(w)
            .next()
            .unwrap()
            .terrain
            .hex_to_world_pos(Hex::new(2, 0))
    };

    move_player(&mut app, edge_pos);
    app.update();

    let w = app.world_mut();
    let in_fov_count = w.query_filtered::<&HCell, With<InFov>>().iter(w).count();

    // hex(2,0) is on the edge — some fov_reach=1 neighbors are outside the grid
    assert!(
        in_fov_count < 7,
        "InFov count at edge hex should be < 7, got {in_fov_count}"
    );
    assert!(
        in_fov_count >= 3,
        "InFov count at edge should be at least 3, got {in_fov_count}"
    );
}

// ── start_fov_transitions + animate_fov_transitions ────────────

#[test]
fn fov_gain_creates_fade_in_transition() {
    let mut cfg = test_config();
    cfg.fov_transition_secs = 10.0; // very slow so transitions don't complete
    let mut app = test_app_with_config(cfg);

    // Move player to trigger fresh InFov additions on newly-visible cells.
    let target_pos = {
        let w = app.world_mut();
        w.query::<&HGrid>()
            .iter(w)
            .next()
            .unwrap()
            .terrain
            .hex_to_world_pos(Hex::new(1, 0))
    };
    move_player(&mut app, target_pos);
    app.update();

    let w = app.world_mut();
    let mut q = w.query::<&FovTransition>();
    let has_fade_in = q.iter(w).any(|tr| tr.direction > 0.0);

    assert!(
        has_fade_in,
        "Some FovTransitions should have direction=1.0 (fade in) after InFov gain"
    );
}

#[test]
fn animate_completes_after_enough_frames() {
    let mut app = test_app();

    // fov_transition_secs = 0.3, dt = 0.1s per frame
    // We need enough frames for progress to reach 1.0 and FovTransition to be removed.
    for _ in 0..5 {
        app.update();
    }

    let w = app.world_mut();
    let remaining = w.query::<&FovTransition>().iter(w).count();
    assert_eq!(
        remaining, 0,
        "All FovTransitions should be complete after enough frames"
    );
}

#[test]
fn fov_loss_reverses_transition_direction() {
    let mut cfg = test_config();
    cfg.fov_transition_secs = 1.0; // slow transition
    let mut app = test_app_with_config(cfg);

    // Run 1 extra frame so transitions have some progress
    app.update();

    // Move player to edge so origin cells lose InFov
    let edge_pos = {
        let w = app.world_mut();
        w.query::<&HGrid>()
            .iter(w)
            .next()
            .unwrap()
            .terrain
            .hex_to_world_pos(Hex::new(2, 0))
    };
    move_player(&mut app, edge_pos);
    app.update();

    let w = app.world_mut();
    let mut q = w.query::<&FovTransition>();
    let has_reversed = q.iter(w).any(|tr| tr.direction < 0.0);

    assert!(
        has_reversed,
        "Some FovTransitions should have direction=-1.0 after InFov loss"
    );
}
