use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, Hex, HexLayout, PlaneMeshBuilder, VertexDirection, shapes};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use bevy_egui::egui;

use super::TerrainConfig;
use super::entities::{
    ActiveHex, DrawnCells, HeightPole, HexCtx, HexEntities, HexGrid, HexSunDisc, LeafCtx,
    NeonMaterials, PetalEdge, PetalRes, QuadLeaf, TriLeaf,
};
use crate::PlayerPos;
use crate::math;

// ── Startup ─────────────────────────────────────────────────────────

/// Spawns the [`HexGrid`] entity, neon materials, and a flat face mesh for every hex cell.
pub fn generate_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<TerrainConfig>,
) {
    // Create neon materials
    let edge_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.5, 1.0),
        emissive: LinearRgba::rgb(0.0, 20.0, 40.0),
        unlit: true,
        ..default()
    });
    let hex_face_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.02, 0.03, 0.05),
        emissive: LinearRgba::rgb(0.02, 0.05, 0.08),
        ..default()
    });
    let gap_face_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.03, 0.05),
        emissive: LinearRgba::rgb(0.03, 0.06, 0.1),
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(NeonMaterials {
        edge_material,
        gap_face_material,
    });

    let g = &cfg.grid;
    let layout = HexLayout {
        scale: Vec2::splat(g.point_spacing),
        ..default()
    };
    let unit_layout = HexLayout {
        scale: Vec2::splat(1.0),
        ..default()
    };

    // Generate noise for heights and sizes
    let height_fbm: Fbm<Perlin> = Fbm::new(g.height_noise_seed).set_octaves(g.height_noise_octaves);
    let radius_fbm: Fbm<Perlin> = Fbm::new(g.radius_noise_seed).set_octaves(g.radius_noise_octaves);
    let mut heights: HashMap<Hex, f32> = HashMap::new();
    let mut radii: HashMap<Hex, f32> = HashMap::new();

    for hex in shapes::hexagon(Hex::ZERO, g.radius) {
        let pos = layout.hex_to_world_pos(hex);

        let noise_val = height_fbm.get([
            pos.x as f64 / g.height_noise_scale,
            pos.y as f64 / g.height_noise_scale,
        ]);
        let h = math::map_noise_to_range(noise_val, 0.0, g.max_height);
        heights.insert(hex, h);

        let radius_noise = radius_fbm.get([
            pos.x as f64 / g.radius_noise_scale,
            pos.y as f64 / g.radius_noise_scale,
        ]);
        let r = math::map_noise_to_range(radius_noise, g.min_hex_radius, g.max_hex_radius);
        radii.insert(hex, r);
    }

    // Compute vertex positions
    let mut vertex_positions: HashMap<(Hex, u8), Vec3> = HashMap::new();
    let unit_offsets = unit_layout.center_aligned_hex_corners();

    for hex in shapes::hexagon(Hex::ZERO, g.radius) {
        let center_2d = layout.hex_to_world_pos(hex);
        let center_height = heights[&hex];
        let radius = radii[&hex];

        for (i, _dir) in VertexDirection::ALL_DIRECTIONS.iter().enumerate() {
            let offset_2d = unit_offsets[i] * radius;
            let world_x = center_2d.x + offset_2d.x;
            let world_z = center_2d.y + offset_2d.y;
            vertex_positions.insert((hex, i as u8), Vec3::new(world_x, center_height, world_z));
        }
    }

    // Spawn hex face meshes
    let hex_mesh_info = PlaneMeshBuilder::new(&unit_layout).build();
    let hex_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, hex_mesh_info.vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, hex_mesh_info.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, hex_mesh_info.uvs)
    .with_inserted_indices(Indices::U16(hex_mesh_info.indices));
    let hex_mesh_handle = meshes.add(hex_mesh);

    let pole_mesh_handle = meshes.add(Cylinder::new(0.5, 1.0));

    let grid_entity = commands
        .spawn((
            Name::new("HexGrid"),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    let mut hex_entity_map: HashMap<Hex, Entity> = HashMap::new();

    for hex in shapes::hexagon(Hex::ZERO, g.radius) {
        let center_2d = layout.hex_to_world_pos(hex);
        let center_height = heights[&hex];
        let radius = radii[&hex];
        let face_height = center_height;

        let entity = commands
            .spawn((
                HexSunDisc { hex },
                Name::new(format!("HexSunDisc({},{})", hex.x, hex.y)),
                Mesh3d(hex_mesh_handle.clone()),
                MeshMaterial3d(hex_face_material.clone()),
                Transform::from_xyz(center_2d.x, face_height, center_2d.y)
                    .with_scale(Vec3::new(radius, 1.0, radius)),
            ))
            .id();
        commands.entity(grid_entity).add_child(entity);
        hex_entity_map.insert(hex, entity);

        // Height indicator pole
        if let Some(pg) = math::pole_geometry(radius, face_height, g.pole_radius_factor, g.pole_gap)
        {
            let pole_radius = pg.radius;
            let pole_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.2),
                emissive: LinearRgba::rgb(0.0, 30.0, 6.0),
                unlit: true,
                ..default()
            });
            let pole_entity = commands
                .spawn((
                    HeightPole,
                    Name::new(format!("Pole({},{})", hex.x, hex.y)),
                    Mesh3d(pole_mesh_handle.clone()),
                    MeshMaterial3d(pole_mat),
                    Transform::from_xyz(0.0, pg.y_center - face_height, 0.0).with_scale(Vec3::new(
                        pole_radius / 0.5 / radius,
                        pg.height,
                        pole_radius / 0.5 / radius,
                    )),
                ))
                .id();
            commands.entity(entity).add_child(pole_entity);
        }
    }

    commands.entity(grid_entity).insert(HexGrid {
        layout,
        heights,
        radii,
        vertex_positions,
    });
    commands.insert_resource(HexEntities {
        map: hex_entity_map,
    });
    commands.init_resource::<ActiveHex>();
}

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
            let terrain_h = interpolate_height(grid, xz);
            player.altitude = cam_tf.translation.y - terrain_h;
        }
    }

    let xz = Vec2::new(player.pos.x, player.pos.z);
    player.pos.y = interpolate_height(grid, xz) + player.altitude;
}

/// Updates [`ActiveHex`] when the player crosses into a new hex.
pub fn track_active_hex(
    grid_q: Query<&HexGrid>,
    hex_entities: Option<Res<HexEntities>>,
    names: Query<&Name>,
    mut cell: ResMut<ActiveHex>,
    player: Res<PlayerPos>,
) {
    let Ok(grid) = grid_q.single() else { return };

    let pos = Vec2::new(player.pos.x, player.pos.z);
    let new_hex = grid.layout.world_pos_to_hex(pos);

    let first_frame = cell.previous.is_none();
    if new_hex != cell.current || first_frame {
        cell.previous = Some(cell.current);
        cell.current = new_hex;
        cell.changed = true;

        if let Some(name) = hex_entities
            .as_ref()
            .and_then(|he| he.map.get(&new_hex))
            .and_then(|&e| names.get(e).ok())
        {
            #[cfg(debug_assertions)]
            println!("Player over: {name}");
        }
    } else {
        cell.changed = false;
    }
}

// ── Update: petal spawning ─────────────────────────────────────────

/// Progressive petal reveal as the player moves.
pub fn spawn_petals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    res: PetalRes,
    mut drawn: ResMut<DrawnCells>,
    mut initial_done: Local<bool>,
) {
    let center = if !*initial_done {
        *initial_done = true;
        Some(Hex::ZERO)
    } else if res.cell.changed {
        Some(res.cell.current)
    } else {
        None
    };

    let Some(center) = center else { return };
    let Ok(grid) = res.grid_q.single() else {
        return;
    };

    let leaf = LeafCtx {
        hex_entities: &res.hex_entities,
        neon: &res.neon,
        grid,
        cfg: &res.cfg,
    };

    for hex in shapes::hexagon(center, res.cfg.petals.reveal_radius) {
        if !grid.heights.contains_key(&hex) || drawn.cells.contains(&hex) {
            continue;
        }
        drawn.cells.insert(hex);

        let Some(&owner_entity) = res.hex_entities.map.get(&hex) else {
            continue;
        };

        let ctx = HexCtx {
            hex,
            owner_entity,
            inverse_tf: world_space_inverse(grid, hex),
        };

        for &edge_idx in &[0u8, 2, 4] {
            spawn_quad_leaf(&mut commands, &mut meshes, &leaf, &ctx, edge_idx);
        }
        for &vtx_idx in &[0u8, 1] {
            spawn_tri_leaf(&mut commands, &mut meshes, &leaf, &ctx, vtx_idx);
        }
    }
}

// ── Update: pole fading ────────────────────────────────────────────

/// Brightens poles near the player and dims distant ones based on horizontal distance.
pub fn highlight_nearby_poles(
    player: Res<PlayerPos>,
    pole_q: Query<(&GlobalTransform, &MeshMaterial3d<StandardMaterial>), With<HeightPole>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<TerrainConfig>,
) {
    let cam_xz = Vec2::new(player.pos.x, player.pos.z);

    for (pole_tf, mat_handle) in &pole_q {
        let pos = pole_tf.translation();
        let pole_xz = Vec2::new(pos.x, pos.z);
        let dist = cam_xz.distance(pole_xz);
        let brightness =
            math::pole_fade_brightness(dist, cfg.grid.pole_fade_distance, cfg.grid.pole_min_alpha);

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

// ── Height interpolation ───────────────────────────────────────────

/// Inverse-distance-weighted height interpolation from nearby hex vertices.
pub fn interpolate_height(grid: &HexGrid, pos: Vec2) -> f32 {
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    let hex = grid.layout.world_pos_to_hex(pos);
    let hexes_to_check: Vec<Hex> = std::iter::once(hex).chain(hex.all_neighbors()).collect();

    for h in hexes_to_check {
        for i in 0..6u8 {
            if let Some(&vpos) = grid.vertex_positions.get(&(h, i)) {
                let dx = pos.x - vpos.x;
                let dz = pos.y - vpos.z;
                let dist_sq = dx * dx + dz * dz;
                if dist_sq < 0.001 {
                    return vpos.y;
                }
                let weight = 1.0 / dist_sq;
                weighted_sum += vpos.y * weight;
                weight_total += weight;
            }
        }
    }

    if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        grid.heights.get(&hex).copied().unwrap_or(0.0)
    }
}

// ── Leaf spawn helpers ─────────────────────────────────────────────

fn spawn_quad_leaf(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    leaf: &LeafCtx,
    ctx: &HexCtx,
    edge_index: u8,
) -> Option<()> {
    let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
    let neighbor = ctx.hex.neighbor(dir);

    leaf.grid.heights.get(&neighbor)?;
    let &neighbor_entity = leaf.hex_entities.map.get(&neighbor)?;

    let vertex_dirs = dir.vertex_directions();
    let v0_idx = vertex_dirs[0].index();
    let v1_idx = vertex_dirs[1].index();

    let opp_dir = dir.const_neg();
    let opp_vertex_dirs = opp_dir.vertex_directions();
    let n0_idx = opp_vertex_dirs[1].index();
    let n1_idx = opp_vertex_dirs[0].index();

    let &va0 = leaf.grid.vertex_positions.get(&(ctx.hex, v0_idx))?;
    let &va1 = leaf.grid.vertex_positions.get(&(ctx.hex, v1_idx))?;
    let &vb0 = leaf.grid.vertex_positions.get(&(neighbor, n0_idx))?;
    let &vb1 = leaf.grid.vertex_positions.get(&(neighbor, n1_idx))?;

    let leaf_name = format!(
        "QuadLeaf({},{})e{}↔({},{})",
        ctx.hex.x, ctx.hex.y, edge_index, neighbor.x, neighbor.y
    );

    let leaf_entity = commands
        .spawn((
            QuadLeaf {
                edge_index,
                neighbor_disc: neighbor_entity,
            },
            Name::new(leaf_name),
            Visibility::default(),
            ctx.inverse_tf,
        ))
        .id();

    // Perimeter edges
    let edge_a = spawn_edge_line(commands, meshes, leaf.neon, leaf.cfg, va0, va1);
    let edge_b = spawn_edge_line(commands, meshes, leaf.neon, leaf.cfg, vb0, vb1);
    commands.entity(leaf_entity).add_children(&[edge_a, edge_b]);

    // Cross-gap edges + quad face
    let cross_a = spawn_edge_line(commands, meshes, leaf.neon, leaf.cfg, va0, vb0);
    let cross_b = spawn_edge_line(commands, meshes, leaf.neon, leaf.cfg, va1, vb1);
    let face = spawn_quad_face(commands, meshes, leaf.neon, va0, va1, vb1, vb0);
    commands
        .entity(leaf_entity)
        .add_children(&[cross_a, cross_b, face]);

    commands.entity(ctx.owner_entity).add_child(leaf_entity);
    Some(())
}

fn spawn_tri_leaf(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    leaf: &LeafCtx,
    ctx: &HexCtx,
    vertex_index: u8,
) -> Option<()> {
    let dir = VertexDirection::ALL_DIRECTIONS[vertex_index as usize];
    let grid_vertex = hexx::GridVertex {
        origin: ctx.hex,
        direction: dir,
    };
    let coords = grid_vertex.coordinates();

    coords
        .iter()
        .all(|c| leaf.grid.heights.contains_key(c))
        .then_some(())?;
    (coords[0] == ctx.hex).then_some(())?;

    let v_idx = dir.index();
    let &v0 = leaf.grid.vertex_positions.get(&(coords[0], v_idx))?;
    let v1 = find_equivalent_vertex(leaf.grid, coords[1], &grid_vertex)?;
    let v2 = find_equivalent_vertex(leaf.grid, coords[2], &grid_vertex)?;

    let &neighbor1_entity = leaf.hex_entities.map.get(&coords[1])?;
    let &neighbor2_entity = leaf.hex_entities.map.get(&coords[2])?;

    let leaf_name = format!(
        "TriLeaf({},{})v{}↔({},{})↔({},{})",
        ctx.hex.x, ctx.hex.y, vertex_index, coords[1].x, coords[1].y, coords[2].x, coords[2].y
    );

    let face_handle = meshes.add(build_tri_mesh(v0, v1, v2));

    let leaf_entity = commands
        .spawn((
            TriLeaf {
                vertex_index,
                neighbor_discs: [neighbor1_entity, neighbor2_entity],
            },
            Name::new(leaf_name),
            Mesh3d(face_handle),
            MeshMaterial3d(leaf.neon.gap_face_material.clone()),
            ctx.inverse_tf,
        ))
        .id();

    commands.entity(ctx.owner_entity).add_child(leaf_entity);
    Some(())
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
    let thickness = cfg.petals.edge_thickness;

    let mesh = meshes.add(Cuboid::new(length, thickness, thickness));
    let direction = diff.normalize();
    let rotation = Quat::from_rotation_arc(Vec3::X, direction);

    commands
        .spawn((
            PetalEdge,
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

fn world_space_inverse(grid: &HexGrid, hex: Hex) -> Transform {
    let center_2d = grid.layout.hex_to_world_pos(hex);
    let height = grid.heights[&hex];
    let radius = grid.radii[&hex];

    let parent_t = Vec3::new(center_2d.x, height, center_2d.y);
    let parent_s = Vec3::new(radius, 1.0, radius);

    Transform {
        translation: Vec3::new(
            -parent_t.x / parent_s.x,
            -parent_t.y / parent_s.y,
            -parent_t.z / parent_s.z,
        ),
        scale: Vec3::new(1.0 / parent_s.x, 1.0 / parent_s.y, 1.0 / parent_s.z),
        ..default()
    }
}

fn find_equivalent_vertex(grid: &HexGrid, hex: Hex, target: &hexx::GridVertex) -> Option<Vec3> {
    for dir in VertexDirection::ALL_DIRECTIONS {
        let candidate = hexx::GridVertex {
            origin: hex,
            direction: dir,
        };
        if candidate.equivalent(target) {
            return grid.vertex_positions.get(&(hex, dir.index())).copied();
        }
    }
    None
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::platform::collections::HashMap;
    use hexx::HexLayout;

    fn single_hex_grid(height: f32) -> HexGrid {
        let layout = HexLayout {
            scale: Vec2::splat(4.0),
            ..default()
        };
        let unit_layout = HexLayout {
            scale: Vec2::splat(1.0),
            ..default()
        };
        let hex = Hex::ZERO;
        let center_2d = layout.hex_to_world_pos(hex);
        let corners = unit_layout.center_aligned_hex_corners();
        let radius = 1.0;

        let mut vertex_positions = HashMap::new();
        for (i, corner) in corners.iter().enumerate() {
            let offset = *corner * radius;
            let world_x = center_2d.x + offset.x;
            let world_z = center_2d.y + offset.y;
            vertex_positions.insert((hex, i as u8), Vec3::new(world_x, height, world_z));
        }

        let mut heights = HashMap::new();
        heights.insert(hex, height);

        HexGrid {
            layout,
            heights,
            radii: HashMap::new(),
            vertex_positions,
        }
    }

    #[test]
    fn interpolate_at_vertex_returns_vertex_height() {
        let grid = single_hex_grid(5.0);
        let vpos = grid.vertex_positions[&(Hex::ZERO, 0)];
        let pos = Vec2::new(vpos.x + 0.0001, vpos.z + 0.0001);
        let h = interpolate_height(&grid, pos);
        assert!(
            (h - 5.0).abs() < 0.1,
            "height near vertex should be ~5.0, got {h}"
        );
    }

    #[test]
    fn interpolate_at_center_returns_vertex_height_when_uniform() {
        let grid = single_hex_grid(3.0);
        let h = interpolate_height(&grid, Vec2::ZERO);
        assert!(
            (h - 3.0).abs() < 0.1,
            "uniform height should be ~3.0, got {h}"
        );
    }

    #[test]
    fn interpolate_outside_grid_falls_back() {
        let grid = single_hex_grid(7.0);
        let h = interpolate_height(&grid, Vec2::new(1000.0, 1000.0));
        assert!(h >= 0.0);
    }
}
