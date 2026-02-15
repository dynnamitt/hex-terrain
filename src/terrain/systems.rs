use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, Hex, VertexDirection, shapes};

use bevy_egui::egui;

use super::TerrainConfig;
use super::entities::{
    FlowerState, HexCtx, HexEntities, HexGrid, HexSunDisc, NeonMaterials, PetalCtx, PetalRes,
    QuadLines, QuadPetal, Stem, TriPetal,
};
use crate::PlayerPos;
use crate::math;

// ── Update: player height + active hex ─────────────────────────────

/// Sets `PlayerPos.pos.y` from terrain interpolation.
///
/// On the first frame, syncs [`PlayerPos::altitude`] from the camera's current
/// Y position so the intro→running transition is seamless.
pub fn update_player_height(
    grid_q: Query<&HexGrid>,
    mut player: ResMut<PlayerPos>,
    cam_q: Query<&Transform, With<crate::drone::Player>>,
    mut synced: Local<bool>,
) {
    let Ok(grid) = grid_q.single() else { return };

    if !*synced {
        *synced = true;
        if let Ok(cam_tf) = cam_q.single() {
            let xz = Vec2::new(cam_tf.translation.x, cam_tf.translation.z);
            let terrain_h = grid.terrain.interpolate_height(xz);
            player.altitude = cam_tf.translation.y - terrain_h;
        }
    }

    let xz = Vec2::new(player.pos.x, player.pos.z);
    player.pos.y = grid.terrain.interpolate_height(xz) + player.altitude;
}

/// Promotes/demotes [`FlowerState`] as the player crosses hex boundaries.
///
/// Uses `Local<Option<Hex>>` to detect hex transitions without a global resource.
pub fn track_player_hex(
    grid_q: Query<&HexGrid>,
    hex_entities: Option<Res<HexEntities>>,
    mut flower_q: Query<&mut FlowerState, With<HexSunDisc>>,
    names: Query<&Name>,
    player: Res<PlayerPos>,
    mut prev_hex: Local<Option<Hex>>,
) {
    let Ok(grid) = grid_q.single() else { return };

    let pos = Vec2::new(player.pos.x, player.pos.z);
    let new_hex = grid.terrain.world_pos_to_hex(pos);

    if *prev_hex == Some(new_hex) {
        return;
    }

    let he = hex_entities.as_ref();

    // Demote old PlayerAbove → Revealed
    if let Some(old_entity) = prev_hex.and_then(|h| he.and_then(|he| he.map.get(&h).copied()))
        && let Ok(mut state) = flower_q.get_mut(old_entity)
        && let FlowerState::PlayerAbove { petals } = &*state
    {
        let petals = petals.clone();
        *state = FlowerState::Revealed { petals };
    }

    // Promote new hex → PlayerAbove
    if let Some(&new_entity) = he.and_then(|he| he.map.get(&new_hex)) {
        if let Ok(mut state) = flower_q.get_mut(new_entity) {
            match &*state {
                FlowerState::Naked => {
                    *state = FlowerState::PlayerAbove { petals: vec![] };
                }
                FlowerState::Revealed { petals } => {
                    let petals = petals.clone();
                    *state = FlowerState::PlayerAbove { petals };
                }
                FlowerState::PlayerAbove { .. } => {}
            }
        }

        if let Ok(name) = names.get(new_entity) {
            #[cfg(debug_assertions)]
            println!("Player over: {name}");
        }
    }

    *prev_hex = Some(new_hex);
}

// ── Update: petal spawning ─────────────────────────────────────────

/// Progressive petal reveal around the player's current hex.
///
/// On the first frame (or when `PlayerAbove` changes hex), iterates the reveal
/// ring and promotes each `Naked` hex to `Revealed`, spawning petal geometry.
/// Also fills petals on a freshly-promoted `PlayerAbove { petals: [] }`.
pub fn reveal_nearby_hexes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    res: PetalRes,
    mut flower_q: Query<(&HexSunDisc, &mut FlowerState)>,
    mut prev_center: Local<Option<Hex>>,
    mut initial_done: Local<bool>,
) {
    // Find the single PlayerAbove hex
    let Some((center_disc, _)) = flower_q
        .iter()
        .find(|(_, s)| matches!(**s, FlowerState::PlayerAbove { .. }))
    else {
        return;
    };
    let center = center_disc.hex;

    let trigger = if !*initial_done {
        *initial_done = true;
        true
    } else {
        *prev_center != Some(center)
    };

    if !trigger {
        return;
    }
    *prev_center = Some(center);

    let Ok(grid) = res.grid_q.single() else {
        return;
    };

    let petal = PetalCtx {
        hex_entities: &res.hex_entities,
        neon: &res.neon,
        grid,
        cfg: &res.cfg,
    };

    for hex in shapes::hexagon(center, res.cfg.flower.reveal_radius) {
        if !grid.terrain.contains(&hex) {
            continue;
        }

        let Some(&owner_entity) = res.hex_entities.map.get(&hex) else {
            continue;
        };

        // Only spawn for Naked hexes or empty PlayerAbove
        let Ok((_, state)) = flower_q.get(owner_entity) else {
            continue;
        };
        let needs_petals = match state {
            FlowerState::Naked => true,
            FlowerState::PlayerAbove { petals } if petals.is_empty() => true,
            _ => false,
        };
        if !needs_petals {
            continue;
        }

        let ctx = HexCtx {
            hex,
            owner_entity,
            inverse_tf: grid.terrain.inverse_transform(hex),
        };

        let mut petals = Vec::new();
        for &edge_idx in &[0u8, 2, 4] {
            if let Some(e) = spawn_quad_petal(&mut commands, &mut meshes, &petal, &ctx, edge_idx) {
                petals.push(e);
            }
        }
        for &vtx_idx in &[0u8, 1] {
            if let Some(e) = spawn_tri_petal(&mut commands, &mut meshes, &petal, &ctx, vtx_idx) {
                petals.push(e);
            }
        }

        // Promote Naked → Revealed, or fill PlayerAbove petals
        if let Ok((_, mut state)) = flower_q.get_mut(owner_entity) {
            match &*state {
                FlowerState::Naked => {
                    *state = FlowerState::Revealed { petals };
                }
                FlowerState::PlayerAbove { petals: cur } if cur.is_empty() => {
                    *state = FlowerState::PlayerAbove { petals };
                }
                _ => {}
            }
        }
    }
}

// ── Update: stem fading ────────────────────────────────────────────

/// Brightens stems near the player and dims distant ones based on horizontal distance.
pub fn highlight_nearby_stems(
    player: Res<PlayerPos>,
    stem_q: Query<(&GlobalTransform, &MeshMaterial3d<StandardMaterial>), With<Stem>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<TerrainConfig>,
) {
    let cam_xz = Vec2::new(player.pos.x, player.pos.z);

    for (stem_tf, mat_handle) in &stem_q {
        let pos = stem_tf.translation();
        let stem_xz = Vec2::new(pos.x, pos.z);
        let dist = cam_xz.distance(stem_xz);
        let brightness = math::stem_fade_brightness(
            dist,
            cfg.flower.stem_fade_distance,
            cfg.flower.stem_min_alpha,
        );

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgb(0.0, brightness, 0.2 * brightness);
            mat.emissive = LinearRgba::rgb(0.0, 30.0 * brightness, 6.0 * brightness);
        }
    }
}

/// Draws the [`Name`] of each [`HexSunDisc`] as a screen-projected egui label.
pub fn draw_hex_labels(
    mut egui_ctx: Query<&mut bevy_egui::EguiContext>,
    camera_q: Query<(&Camera, &GlobalTransform), With<crate::drone::Player>>,
    hexes: Query<(&GlobalTransform, &Name), With<HexSunDisc>>,
    mut ready: Local<bool>,
) {
    if !*ready {
        *ready = true;
        return;
    }
    let Ok((camera, cam_gt)) = camera_q.single() else {
        return;
    };
    let Ok(mut ctx) = egui_ctx.single_mut() else {
        return;
    };
    let cam_pos = cam_gt.translation();

    let painter = ctx.get_mut().layer_painter(egui::LayerId::background());

    for (hex_gt, name) in &hexes {
        let world_pos = hex_gt.translation();
        if cam_pos.distance(world_pos) > 30.0 {
            continue;
        }
        if let Ok(viewport) = camera.world_to_viewport(cam_gt, world_pos) {
            painter.text(
                egui::pos2(viewport.x, viewport.y),
                egui::Align2::CENTER_CENTER,
                name.as_str(),
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
        }
    }
}

// ── Petal spawn helpers ────────────────────────────────────────────

fn spawn_quad_petal(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    petal: &PetalCtx,
    ctx: &HexCtx,
    edge_index: u8,
) -> Option<Entity> {
    let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
    let neighbor = ctx.hex.neighbor(dir);

    petal.grid.terrain.height(&neighbor)?;
    let &neighbor_entity = petal.hex_entities.map.get(&neighbor)?;

    let vertex_dirs = dir.vertex_directions();
    let v0_idx = vertex_dirs[0].index();
    let v1_idx = vertex_dirs[1].index();

    let opp_dir = dir.const_neg();
    let opp_vertex_dirs = opp_dir.vertex_directions();
    let n0_idx = opp_vertex_dirs[1].index();
    let n1_idx = opp_vertex_dirs[0].index();

    let va0 = petal.grid.terrain.vertex(ctx.hex, v0_idx)?;
    let va1 = petal.grid.terrain.vertex(ctx.hex, v1_idx)?;
    let vb0 = petal.grid.terrain.vertex(neighbor, n0_idx)?;
    let vb1 = petal.grid.terrain.vertex(neighbor, n1_idx)?;

    let petal_name = format!(
        "QuadPetal({},{})e{}↔({},{})",
        ctx.hex.x, ctx.hex.y, edge_index, neighbor.x, neighbor.y
    );

    let petal_entity = commands
        .spawn((
            QuadPetal {
                edge_index,
                neighbor_disc: neighbor_entity,
            },
            Name::new(petal_name),
            Visibility::default(),
            ctx.inverse_tf,
        ))
        .id();

    // Perimeter edges
    let edge_a = spawn_edge_line(commands, meshes, petal.neon, petal.cfg, va0, va1);
    let edge_b = spawn_edge_line(commands, meshes, petal.neon, petal.cfg, vb0, vb1);
    commands
        .entity(petal_entity)
        .add_children(&[edge_a, edge_b]);

    // Cross-gap edges + quad face
    let cross_a = spawn_edge_line(commands, meshes, petal.neon, petal.cfg, va0, vb0);
    let cross_b = spawn_edge_line(commands, meshes, petal.neon, petal.cfg, va1, vb1);
    let face = spawn_quad_face(commands, meshes, petal.neon, va0, va1, vb1, vb0);
    commands
        .entity(petal_entity)
        .add_children(&[cross_a, cross_b, face]);

    commands.entity(ctx.owner_entity).add_child(petal_entity);
    Some(petal_entity)
}

fn spawn_tri_petal(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    petal: &PetalCtx,
    ctx: &HexCtx,
    vertex_index: u8,
) -> Option<Entity> {
    let dir = VertexDirection::ALL_DIRECTIONS[vertex_index as usize];
    let grid_vertex = hexx::GridVertex {
        origin: ctx.hex,
        direction: dir,
    };
    let coords = grid_vertex.coordinates();

    coords
        .iter()
        .all(|c| petal.grid.terrain.contains(c))
        .then_some(())?;
    (coords[0] == ctx.hex).then_some(())?;

    let v_idx = dir.index();
    let v0 = petal.grid.terrain.vertex(coords[0], v_idx)?;
    let v1 = petal
        .grid
        .terrain
        .find_equivalent_vertex(coords[1], &grid_vertex)?;
    let v2 = petal
        .grid
        .terrain
        .find_equivalent_vertex(coords[2], &grid_vertex)?;

    let &neighbor1_entity = petal.hex_entities.map.get(&coords[1])?;
    let &neighbor2_entity = petal.hex_entities.map.get(&coords[2])?;

    let petal_name = format!(
        "TriPetal({},{})v{}↔({},{})↔({},{})",
        ctx.hex.x, ctx.hex.y, vertex_index, coords[1].x, coords[1].y, coords[2].x, coords[2].y
    );

    let face_handle = meshes.add(build_tri_mesh(v0, v1, v2));

    let petal_entity = commands
        .spawn((
            TriPetal {
                vertex_index,
                neighbor_discs: [neighbor1_entity, neighbor2_entity],
            },
            Name::new(petal_name),
            Mesh3d(face_handle),
            MeshMaterial3d(petal.neon.gap_face_material.clone()),
            ctx.inverse_tf,
        ))
        .id();

    commands.entity(ctx.owner_entity).add_child(petal_entity);
    Some(petal_entity)
}

// ── Mesh spawn helpers ─────────────────────────────────────────────

fn spawn_edge_line(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    neon: &NeonMaterials,
    cfg: &TerrainConfig,
    from: Vec3,
    to: Vec3,
) -> Entity {
    let midpoint = (from + to) / 2.0;
    let diff = to - from;
    let length = diff.length();
    let thickness = cfg.flower.edge_thickness;

    let mesh = meshes.add(Cuboid::new(length, thickness, thickness));
    let direction = diff.normalize();
    let rotation = Quat::from_rotation_arc(Vec3::X, direction);

    commands
        .spawn((
            QuadLines,
            Mesh3d(mesh),
            MeshMaterial3d(neon.edge_material.clone()),
            Transform::from_translation(midpoint).with_rotation(rotation),
        ))
        .id()
}

fn spawn_quad_face(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    neon: &NeonMaterials,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    v3: Vec3,
) -> Entity {
    let positions = vec![v0.to_array(), v1.to_array(), v2.to_array(), v3.to_array()];
    let normal = math::compute_normal(v0, v1, v2);
    let normals = vec![normal.to_array(); 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0u16, 1, 2, 0, 2, 3];

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U16(indices));

    commands
        .spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(neon.gap_face_material.clone()),
        ))
        .id()
}

// ── Pure helpers ───────────────────────────────────────────────────

fn build_tri_mesh(v0: Vec3, v1: Vec3, v2: Vec3) -> Mesh {
    let positions = vec![v0.to_array(), v1.to_array(), v2.to_array()];
    let normal = math::compute_normal(v0, v1, v2);
    let normals = vec![normal.to_array(); 3];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
    let indices = vec![0u16, 1, 2];

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U16(indices))
}
