//! Quad and Tri gap geometry: spawning, mesh construction, index math.

use bevy::asset::RenderAssetUsages;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
use bevy::mesh::Indices;
use bevy::picking::mesh_picking::ray_cast::RayCastBackfaces;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, Hex, VertexDirection};

use super::entities::{
    HCell, Quad, QuadEdge, QuadOwner, QuadPos1Emitter, QuadPos2Emitter, QuadTail, Tri, TriOwner,
    TriPos1Emitter, TriPos2Emitter,
};
use super::h_grid_layout::HGridLayout;
use super::math;

/// Spawns a quad gap mesh bridging an even edge between `hex` and its neighbor.
///
/// The quad's four corners come from two vertices on `hex` and two on the
/// neighbor across `edge_index` (must be 0, 2, or 4). The mesh is parented
/// to the owner corner, and marker components ([`QuadOwner`], [`QuadTail`],
/// [`QuadPos1Emitter`], [`QuadPos2Emitter`]) are inserted on the four
/// participating [`Corner`](super::entities::Corner) entities so downstream
/// systems can navigate from corner to gap mesh without hierarchy traversal.
///
/// Four emissive [`QuadEdge`] cuboids are spawned as children of the mesh.
///
/// Returns `None` (no-op) when the neighbor or any corner entity is missing,
/// which happens for hexes on the grid boundary.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_quad(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    gap_material: &Handle<StandardMaterial>,
    edge_material: &Handle<StandardMaterial>,
    terrain: &HGridLayout,
    corner_entities: &HashMap<(Hex, u8), Entity>,
    hex_entities: &HashMap<Hex, Entity>,
    hex: Hex,
    edge_index: u8,
) -> Option<()> {
    let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
    let neighbor = hex.neighbor(dir);
    let &neighbor_hex_entity = hex_entities.get(&neighbor)?;

    let (v0_idx, v1_idx, n0_idx, n1_idx) = quad_corner_indices(edge_index);

    // All 4 corner entities must exist (grid-edge guard)
    let &owner_entity = corner_entities.get(&(hex, v0_idx))?;
    let &tail_entity = corner_entities.get(&(hex, v1_idx))?;
    let &pos2_entity = corner_entities.get(&(neighbor, n0_idx))?;
    let &pos3_entity = corner_entities.get(&(neighbor, n1_idx))?;

    // All 4 vertex positions
    let v0 = terrain.vertex(hex, v0_idx)?;
    let v1 = terrain.vertex(neighbor, n0_idx)?;
    let v2 = terrain.vertex(neighbor, n1_idx)?;
    let v3 = terrain.vertex(hex, v1_idx)?;

    // Build mesh in corner-local space
    let mesh = build_gap_mesh(&[v0, v1, v2, v3]);
    let mesh_entity = commands
        .spawn((
            Quad,
            RayCastBackfaces,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(gap_material.clone()),
            Transform::default(),
        ))
        .id();
    commands.entity(owner_entity).add_child(mesh_entity);

    // Add marker components to corner entities
    commands.entity(owner_entity).insert(QuadOwner {
        gap: mesh_entity,
        neighbor_hex: neighbor_hex_entity,
    });
    commands
        .entity(pos2_entity)
        .insert(QuadPos1Emitter(mesh_entity));
    commands
        .entity(pos3_entity)
        .insert(QuadPos2Emitter(mesh_entity));
    commands.entity(tail_entity).insert(QuadTail);

    // Spawn edge lines as children of the Quad mesh entity
    let edge_thickness = 0.03;
    let origin = v0;
    let edges = [(v0, v3), (v1, v2), (v0, v1), (v3, v2)];
    for (from, to) in edges {
        let local_from = from - origin;
        let local_to = to - origin;
        let (midpoint, length, rotation) = math::edge_cuboid_transform(local_from, local_to);
        let edge_entity = commands
            .spawn((
                QuadEdge,
                Mesh3d(meshes.add(Cuboid::new(length, edge_thickness, edge_thickness))),
                MeshMaterial3d(edge_material.clone()),
                Transform::from_translation(midpoint).with_rotation(rotation),
            ))
            .id();
        commands.entity(mesh_entity).add_child(edge_entity);
    }

    Some(())
}

/// Spawns a triangular gap mesh at a vertex junction shared by three hexes.
///
/// Each vertex junction is claimed by exactly one hex via canonical ownership:
/// only the hex that equals `GridVertex::coordinates()[0]` spawns the tri.
/// `vertex_index` must be 0 or 1 — these are the only indices where the
/// current hex can be the canonical owner.
///
/// The mesh is parented to the owner corner, and marker components
/// ([`TriOwner`], [`TriPos1Emitter`], [`TriPos2Emitter`]) are inserted on
/// the three participating [`Corner`](super::entities::Corner) entities.
///
/// Returns `None` when this hex is not the canonical owner, or when any of
/// the three neighboring corners are missing (grid boundary).
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_tri(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    gap_material: &Handle<StandardMaterial>,
    terrain: &HGridLayout,
    corner_entities: &HashMap<(Hex, u8), Entity>,
    hex_entities: &HashMap<Hex, Entity>,
    hex: Hex,
    vertex_index: u8,
) -> Option<()> {
    let dir = VertexDirection::ALL_DIRECTIONS[vertex_index as usize];
    let grid_vertex = hexx::GridVertex {
        origin: hex,
        direction: dir,
    };
    let coords = grid_vertex.coordinates();

    // Canonical ownership: this hex must be coords[0]
    (coords[0] == hex).then_some(())?;

    let &neighbor1_hex_entity = hex_entities.get(&coords[1])?;
    let &neighbor2_hex_entity = hex_entities.get(&coords[2])?;

    let v0_idx = dir.index();
    let idx1 = corner_index_for_vertex(coords[1], &grid_vertex)?;
    let idx2 = corner_index_for_vertex(coords[2], &grid_vertex)?;

    // All 3 corner entities must exist
    let &owner_entity = corner_entities.get(&(hex, v0_idx))?;
    let &pos1_entity = corner_entities.get(&(coords[1], idx1))?;
    let &pos2_entity = corner_entities.get(&(coords[2], idx2))?;

    // All 3 vertex positions
    let v0 = terrain.vertex(coords[0], v0_idx)?;
    let v1 = terrain.vertex(coords[1], idx1)?;
    let v2 = terrain.vertex(coords[2], idx2)?;

    // Build mesh in corner-local space
    let mesh = build_gap_mesh(&[v0, v1, v2]);
    let mesh_entity = commands
        .spawn((
            Tri,
            RayCastBackfaces,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(gap_material.clone()),
            Transform::default(),
        ))
        .id();
    commands.entity(owner_entity).add_child(mesh_entity);

    // Add marker components to corner entities
    commands.entity(owner_entity).insert(TriOwner {
        gap: mesh_entity,
        neighbor1_hex: neighbor1_hex_entity,
        neighbor2_hex: neighbor2_hex_entity,
    });
    commands
        .entity(pos1_entity)
        .insert(TriPos1Emitter(mesh_entity));
    commands
        .entity(pos2_entity)
        .insert(TriPos2Emitter(mesh_entity));
    Some(())
}

/// Maps an even edge index (0, 2, or 4) to the four corner indices that
/// form the quad gap across that edge.
///
/// Returns `(v0, v1, n0, n1)` where `v0`/`v1` are vertex indices on the
/// owning hex and `n0`/`n1` are vertex indices on the neighbor. The winding
/// is chosen so that `v0` is the [`QuadOwner`] corner and `v1` is the
/// [`QuadTail`], while `n0`/`n1` map to [`QuadPos1Emitter`]/[`QuadPos2Emitter`].
fn quad_corner_indices(edge_index: u8) -> (u8, u8, u8, u8) {
    let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
    let vertex_dirs = dir.vertex_directions();
    let v0_idx = vertex_dirs[0].index();
    let v1_idx = vertex_dirs[1].index();

    let opp_dir = dir.const_neg();
    let opp_vertex_dirs = opp_dir.vertex_directions();
    let n0_idx = opp_vertex_dirs[1].index();
    let n1_idx = opp_vertex_dirs[0].index();

    (v0_idx, v1_idx, n0_idx, n1_idx)
}

/// Constructs a triangle (3 verts) or quad (4 verts) [`Mesh`] from world-space
/// positions, translated into the first vertex's local space.
///
/// The mesh includes position, normal, and UV attributes, plus index data.
/// `MAIN_WORLD` asset usage is set so the mesh is available for
/// [`MeshRayCast`](bevy::picking::mesh_picking::ray_cast::MeshRayCast) hits.
fn build_gap_mesh(world_verts: &[Vec3]) -> Mesh {
    let (positions, normal) = math::gap_vertex_data(world_verts);
    let normals = vec![normal; positions.len()];

    let (uvs, indices): (Vec<[f32; 2]>, Vec<u16>) = if world_verts.len() == 4 {
        (
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![0, 1, 2, 0, 2, 3],
        )
    } else {
        (vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]], vec![0, 1, 2])
    };

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U16(indices))
}

/// Bundles ECS queries needed to update a single vertex of a gap mesh at runtime.
#[derive(SystemParam)]
pub(super) struct GapMeshAccess<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
    meshes: ResMut<'w, Assets<Mesh>>,
    mesh_handles: Query<'w, 's, &'static Mesh3d>,
    children: Query<'w, 's, &'static Children>,
    edge_transforms: Query<'w, 's, &'static mut Transform, (With<QuadEdge>, Without<HCell>)>,
}

impl GapMeshAccess<'_, '_> {
    /// Updates a single vertex of a gap mesh to a new Y world coordinate.
    ///
    /// Reads the mesh's current positions (in owner-corner-local space),
    /// converts to world space, sets `world_verts[vertex_index].y = new_y`,
    /// then recomputes local positions and normals via [`math::gap_vertex_data`].
    /// Also repositions any [`QuadEdge`] children to match the updated geometry.
    ///
    /// Returns `None` if any entity or asset lookup fails.
    pub fn realign_neighboring_vertex(
        &mut self,
        gap: Entity,
        vertex_index: u8,
        new_y: f32,
    ) -> Option<()> {
        let owner = self.parents.get(gap).ok()?.get();
        let owner_world = self.transforms.get(owner).ok()?.translation();

        let handle = &self.mesh_handles.get(gap).ok()?.0;
        let mesh = self.meshes.get(handle)?;
        let positions: Vec<[f32; 3]> = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)?
            .as_float3()?
            .to_vec();

        let mut world_verts: Vec<Vec3> = positions
            .iter()
            .map(|&p| owner_world + Vec3::from(p))
            .collect();
        world_verts[vertex_index as usize].y = new_y;

        let (new_positions, normal) = math::gap_vertex_data(&world_verts);
        let normals = vec![normal; new_positions.len()];

        let handle = &self.mesh_handles.get(gap).ok()?.0;
        let mesh = self.meshes.get_mut(handle)?;
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

        self.reposition_edges(gap);

        Some(())
    }

    /// Shifts a single vertex's Y by `delta_y` in owner-corner-local space.
    ///
    /// Used for owned gap meshes when the owning hex lowers: neighbor vertices
    /// must shift up in local space to maintain their world position.
    /// Also repositions any [`QuadEdge`] children to match the updated geometry.
    pub fn shift_vertex_y(&mut self, gap: Entity, vertex_index: u8, delta_y: f32) -> Option<()> {
        let handle = &self.mesh_handles.get(gap).ok()?.0;
        let mesh = self.meshes.get(handle)?;
        let mut positions: Vec<[f32; 3]> = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)?
            .as_float3()?
            .to_vec();

        positions[vertex_index as usize][1] += delta_y;

        // positions[0] is always [0,0,0] so they work as pseudo-world coords
        let world_verts: Vec<Vec3> = positions.iter().map(|&p| Vec3::from(p)).collect();
        let (new_positions, normal) = math::gap_vertex_data(&world_verts);
        let normals = vec![normal; new_positions.len()];

        let handle = &self.mesh_handles.get(gap).ok()?.0;
        let mesh = self.meshes.get_mut(handle)?;
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

        self.reposition_edges(gap);

        Some(())
    }

    /// Recomputes [`QuadEdge`] transforms and meshes from the parent gap's
    /// current vertex positions. No-op for Tri meshes (no edge children).
    fn reposition_edges(&mut self, gap: Entity) {
        let handle = &self.mesh_handles.get(gap).ok();
        let Some(handle) = handle else { return };
        let Some(mesh) = self.meshes.get(&handle.0) else {
            return;
        };
        let Some(positions) = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
        else {
            return;
        };
        let positions: Vec<[f32; 3]> = positions.to_vec();

        // Only quads have edges (4 vertices)
        if positions.len() != 4 {
            return;
        }

        let Ok(gap_children) = self.children.get(gap) else {
            return;
        };
        let edge_entities: Vec<Entity> = gap_children
            .iter()
            .filter(|&e| self.edge_transforms.contains(e))
            .collect();
        if edge_entities.is_empty() {
            return;
        }

        let p: Vec<Vec3> = positions.iter().map(|p| Vec3::from_array(*p)).collect();
        let edge_thickness = 0.03;
        let edges = [(p[0], p[3]), (p[1], p[2]), (p[0], p[1]), (p[3], p[2])];

        for (i, (from, to)) in edges.iter().enumerate() {
            let Some(&edge_entity) = edge_entities.get(i) else {
                break;
            };
            let (midpoint, length, rotation) = math::edge_cuboid_transform(*from, *to);

            if let Ok(mut tf) = self.edge_transforms.get_mut(edge_entity) {
                *tf = Transform::from_translation(midpoint).with_rotation(rotation);
            }

            if let Ok(mesh3d) = self.mesh_handles.get(edge_entity)
                && let Some(edge_mesh) = self.meshes.get_mut(&mesh3d.0)
            {
                *edge_mesh = Cuboid::new(length, edge_thickness, edge_thickness).into();
            }
        }
    }
}

/// Finds which corner index (0..6) on `hex` shares the same vertex junction
/// as `target`. Returns `None` if `hex` does not participate in that junction.
fn corner_index_for_vertex(hex: Hex, target: &hexx::GridVertex) -> Option<u8> {
    VertexDirection::ALL_DIRECTIONS.iter().find_map(|&dir| {
        let candidate = hexx::GridVertex {
            origin: hex,
            direction: dir,
        };
        candidate.equivalent(target).then_some(dir.index())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_corner_edge0() {
        let (v0, v1, n0, n1) = quad_corner_indices(0);
        let dir = EdgeDirection::ALL_DIRECTIONS[0];
        let vd = dir.vertex_directions();
        let opp = dir.const_neg();
        let od = opp.vertex_directions();
        assert_eq!(
            (v0, v1, n0, n1),
            (vd[0].index(), vd[1].index(), od[1].index(), od[0].index())
        );
    }

    #[test]
    fn quad_corner_edge2() {
        let (v0, v1, n0, n1) = quad_corner_indices(2);
        let dir = EdgeDirection::ALL_DIRECTIONS[2];
        let vd = dir.vertex_directions();
        let opp = dir.const_neg();
        let od = opp.vertex_directions();
        assert_eq!(
            (v0, v1, n0, n1),
            (vd[0].index(), vd[1].index(), od[1].index(), od[0].index())
        );
    }

    #[test]
    fn quad_corner_edge4() {
        let (v0, v1, n0, n1) = quad_corner_indices(4);
        let dir = EdgeDirection::ALL_DIRECTIONS[4];
        let vd = dir.vertex_directions();
        let opp = dir.const_neg();
        let od = opp.vertex_directions();
        assert_eq!(
            (v0, v1, n0, n1),
            (vd[0].index(), vd[1].index(), od[1].index(), od[0].index())
        );
    }

    #[test]
    fn quad_corner_owner_indices_distinct() {
        for edge in [0u8, 2, 4] {
            let (v0, v1, _, _) = quad_corner_indices(edge);
            assert_ne!(v0, v1, "edge {edge}: owner indices must be distinct");
        }
    }

    #[test]
    fn quad_corner_neighbor_indices_distinct() {
        for edge in [0u8, 2, 4] {
            let (_, _, n0, n1) = quad_corner_indices(edge);
            assert_ne!(n0, n1, "edge {edge}: neighbor indices must be distinct");
        }
    }

    // ── Test helpers ───────────────────────────────────────────────

    use bevy::asset::AssetPlugin;
    use bevy::ecs::system::RunSystemOnce;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>();
        app.update();
        app
    }

    /// Reads mesh positions from an entity's Mesh3d handle.
    fn read_positions(app: &App, entity: Entity) -> Vec<[f32; 3]> {
        let meshes = app.world().resource::<Assets<Mesh>>();
        let handle = &app.world().get::<Mesh3d>(entity).unwrap().0;
        meshes
            .get(handle)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap()
            .to_vec()
    }

    /// Reads mesh normals from an entity's Mesh3d handle.
    fn read_normals(app: &App, entity: Entity) -> Vec<[f32; 3]> {
        let meshes = app.world().resource::<Assets<Mesh>>();
        let handle = &app.world().get::<Mesh3d>(entity).unwrap().0;
        meshes
            .get(handle)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .unwrap()
            .as_float3()
            .unwrap()
            .to_vec()
    }

    /// Spawns a gap mesh as child of an owner corner at `v0`.
    /// Returns `(owner, gap)`.
    fn spawn_gap(app: &mut App, world_verts: &[Vec3]) -> (Entity, Entity) {
        let v0 = world_verts[0];
        let owner = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(v0))
            .id();
        let mesh = build_gap_mesh(world_verts);
        let mesh_handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
        let gap = app.world_mut().spawn(Mesh3d(mesh_handle)).id();
        app.world_mut().entity_mut(owner).add_child(gap);
        (owner, gap)
    }

    /// Spawns 4 QuadEdge children on a gap, matching the spawn order in `spawn_quad`.
    fn spawn_quad_edges(app: &mut App, gap: Entity, positions: &[[f32; 3]; 4]) -> [Entity; 4] {
        let p: Vec<Vec3> = positions.iter().map(|p| Vec3::from_array(*p)).collect();
        let edge_thickness = 0.03;
        let pairs = [(p[0], p[3]), (p[1], p[2]), (p[0], p[1]), (p[3], p[2])];
        let mut edges = [Entity::PLACEHOLDER; 4];
        for (i, (from, to)) in pairs.iter().enumerate() {
            let (midpoint, length, rotation) = math::edge_cuboid_transform(*from, *to);
            let mesh = app
                .world_mut()
                .resource_mut::<Assets<Mesh>>()
                .add(Cuboid::new(length, edge_thickness, edge_thickness));
            let edge = app
                .world_mut()
                .spawn((
                    QuadEdge,
                    Mesh3d(mesh),
                    Transform::from_translation(midpoint).with_rotation(rotation),
                ))
                .id();
            app.world_mut().entity_mut(gap).add_child(edge);
            edges[i] = edge;
        }
        edges
    }

    // ── Tests ───────────────────────────────────────────────────────

    #[test]
    fn realign_neighboring_vertex_updates_mesh() {
        let mut app = test_app();

        let v0 = Vec3::new(0.0, 1.0, 0.0);
        let v1 = Vec3::new(1.0, 1.0, 0.0);
        let v2 = Vec3::new(0.5, 1.0, 1.0);
        let (_, gap) = spawn_gap(&mut app, &[v0, v1, v2]);

        let new_y = 5.0;
        let _ = app
            .world_mut()
            .run_system_once(move |mut gap_mesh: GapMeshAccess| {
                let result = gap_mesh.realign_neighboring_vertex(gap, 1, new_y);
                assert!(
                    result.is_some(),
                    "realign_neighboring_vertex should succeed"
                );
            });

        let expected_world = [v0, Vec3::new(v1.x, new_y, v1.z), v2];
        let (expected_positions, expected_normal) = math::gap_vertex_data(&expected_world);

        assert_eq!(read_positions(&app, gap), expected_positions);
        assert_eq!(read_normals(&app, gap), vec![expected_normal; 3]);
    }

    #[test]
    fn shift_vertex_y_updates_tri_mesh() {
        let mut app = test_app();

        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.5, 0.0, 1.0);
        let (_, gap) = spawn_gap(&mut app, &[v0, v1, v2]);

        let delta = 3.0;
        let _ = app
            .world_mut()
            .run_system_once(move |mut gm: GapMeshAccess| {
                assert!(gm.shift_vertex_y(gap, 1, delta).is_some());
            });

        // Vertex 1 Y shifted by delta; others unchanged
        let expected_world = [v0, Vec3::new(v1.x, delta, v1.z), v2];
        let (expected_positions, expected_normal) = math::gap_vertex_data(&expected_world);

        assert_eq!(read_positions(&app, gap), expected_positions);
        assert_eq!(read_normals(&app, gap), vec![expected_normal; 3]);
    }

    #[test]
    fn shift_vertex_y_repositions_quad_edges() {
        let mut app = test_app();

        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(2.0, 0.0, 0.0);
        let v2 = Vec3::new(2.0, 0.0, 2.0);
        let v3 = Vec3::new(0.0, 0.0, 2.0);
        let (_, gap) = spawn_gap(&mut app, &[v0, v1, v2, v3]);

        let initial_positions: [[f32; 3]; 4] = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 0.0, 2.0],
            [0.0, 0.0, 2.0],
        ];
        let edge_entities = spawn_quad_edges(&mut app, gap, &initial_positions);

        // Record edge 2's (p0→p1) original transform
        let original_tf = *app.world().get::<Transform>(edge_entities[2]).unwrap();

        let delta = 4.0;
        let _ = app
            .world_mut()
            .run_system_once(move |mut gm: GapMeshAccess| {
                assert!(gm.shift_vertex_y(gap, 1, delta).is_some());
            });

        // Edge 2 connects p[0] to p[1]; p[1].y shifted by delta → new transform
        let updated_tf = *app.world().get::<Transform>(edge_entities[2]).unwrap();
        assert_ne!(
            original_tf.translation, updated_tf.translation,
            "edge 2 midpoint should move after vertex shift"
        );

        // Verify the new transform matches edge_cuboid_transform for updated verts
        let new_p1 = Vec3::new(2.0, delta, 0.0);
        let (expected_mid, expected_len, expected_rot) =
            math::edge_cuboid_transform(Vec3::ZERO, new_p1);
        assert!(
            (updated_tf.translation - expected_mid).length() < 1e-5,
            "edge midpoint: expected {expected_mid}, got {}",
            updated_tf.translation
        );
        assert!(
            updated_tf.rotation.angle_between(expected_rot) < 1e-3,
            "edge rotation should match"
        );

        // Edge mesh should be a cuboid with the new length
        let meshes = app.world().resource::<Assets<Mesh>>();
        let edge_handle = &app.world().get::<Mesh3d>(edge_entities[2]).unwrap().0;
        let edge_mesh = meshes.get(edge_handle).unwrap();
        let edge_positions = edge_mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        // Cuboid half-extents: x = length/2, y = thickness/2, z = thickness/2
        let max_x = edge_positions
            .iter()
            .map(|p| p[0].abs())
            .fold(0.0f32, f32::max);
        assert!(
            (max_x - expected_len / 2.0).abs() < 1e-4,
            "edge mesh half-length: expected {}, got {max_x}",
            expected_len / 2.0
        );
    }

    #[test]
    fn realign_repositions_quad_edges() {
        let mut app = test_app();

        let v0 = Vec3::new(0.0, 1.0, 0.0);
        let v1 = Vec3::new(2.0, 1.0, 0.0);
        let v2 = Vec3::new(2.0, 1.0, 2.0);
        let v3 = Vec3::new(0.0, 1.0, 2.0);
        let (_, gap) = spawn_gap(&mut app, &[v0, v1, v2, v3]);

        // Initial local positions after build_gap_mesh
        let initial_positions: [[f32; 3]; 4] = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 0.0, 2.0],
            [0.0, 0.0, 2.0],
        ];
        let edge_entities = spawn_quad_edges(&mut app, gap, &initial_positions);

        let original_tf = *app.world().get::<Transform>(edge_entities[2]).unwrap();

        let new_y = 5.0;
        let _ = app
            .world_mut()
            .run_system_once(move |mut gm: GapMeshAccess| {
                assert!(gm.realign_neighboring_vertex(gap, 1, new_y).is_some());
            });

        let updated_tf = *app.world().get::<Transform>(edge_entities[2]).unwrap();
        assert_ne!(
            original_tf.translation, updated_tf.translation,
            "edge 2 should reposition after realign"
        );
    }
}
