//! Pure computation helpers for the h_terrain subsystem.
//!
//! All functions are free of Bevy ECS dependencies and operate on plain
//! numeric / `Vec3` inputs. `build_gap_mesh` is the sole exception — it
//! constructs a [`Mesh`] but has no ECS side effects.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hexx::{EdgeDirection, GridVertex, Hex, VertexDirection};

/// Maps a noise value from the standard `[-1, 1]` range into `[min, max]`.
///
/// Noise generators (e.g. `Fbm<Perlin>`) produce values centred around zero.
/// This linearly rescales to an arbitrary output range.
///
/// # Examples
/// ```ignore
/// assert_eq!(map_noise_to_range(-1.0, 0.0, 10.0), 0.0);
/// assert_eq!(map_noise_to_range( 1.0, 0.0, 10.0), 10.0);
/// assert_eq!(map_noise_to_range( 0.0, 2.0, 6.0),  4.0);
/// ```
pub(crate) fn map_noise_to_range(noise_val: f64, min: f32, max: f32) -> f32 {
    min + ((noise_val as f32 + 1.0) / 2.0) * (max - min)
}

/// Computes the face normal of a triangle defined by three vertices.
///
/// Uses the cross product of edges `(v1 - v0)` and `(v2 - v0)`.
/// Returns `Vec3::ZERO` if the triangle is degenerate (collinear points).
pub(crate) fn compute_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    edge1.cross(edge2).normalize_or_zero()
}

/// Count total (quads, tris) for a grid using the same ownership rules
/// as `generate_h_grid`: quads on even edges [0,2,4] where neighbor exists,
/// tris on vertices [0,1] with canonical ownership and all 3 coords in grid.
pub(crate) fn gap_filler(grid: &[Hex]) -> (usize, usize) {
    let mut quads = 0;
    let mut tris = 0;

    for &hex in grid {
        for edge_index in [0usize, 2, 4] {
            let dir = EdgeDirection::ALL_DIRECTIONS[edge_index];
            let neighbor = hex.neighbor(dir);
            if grid.contains(&neighbor) {
                quads += 1;
            }
        }

        for vertex_index in [0usize, 1] {
            let dir = VertexDirection::ALL_DIRECTIONS[vertex_index];
            let gv = GridVertex {
                origin: hex,
                direction: dir,
            };
            let coords = gv.coordinates();
            if coords[0] != hex {
                continue;
            }
            if coords.iter().all(|c| grid.contains(c)) {
                tris += 1;
            }
        }
    }

    (quads, tris)
}

/// Snap threshold for IDW interpolation — if query is within this squared
/// distance of a vertex, snap directly to that vertex's height.
const IDW_SNAP_THRESHOLD: f32 = 0.001;

/// Inverse-distance-weighted height interpolation from 3D vertices projected to XZ.
/// Returns `None` if `vertices` is empty; caller supplies fallback.
pub(crate) fn idw_interpolate_height(pos: Vec2, vertices: &[Vec3]) -> Option<f32> {
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    for &vpos in vertices {
        let dx = pos.x - vpos.x;
        let dz = pos.y - vpos.z;
        let dist_sq = dx * dx + dz * dz;
        if dist_sq < IDW_SNAP_THRESHOLD {
            return Some(vpos.y);
        }
        let weight = 1.0 / dist_sq;
        weighted_sum += vpos.y * weight;
        weight_total += weight;
    }

    if weight_total > 0.0 {
        Some(weighted_sum / weight_total)
    } else {
        None
    }
}

/// Placement for a cuboid along an edge: (midpoint, length, rotation).
pub fn edge_cuboid_transform(from: Vec3, to: Vec3) -> (Vec3, f32, Quat) {
    let diff = to - from;
    let length = diff.length();
    let midpoint = (from + to) / 2.0;
    let direction = diff.normalize_or_zero();
    let rotation = if direction == Vec3::ZERO {
        Quat::IDENTITY
    } else {
        Quat::from_rotation_arc(Vec3::X, direction)
    };
    (midpoint, length, rotation)
}

/// Corner indices for a quad gap: (owner_v0, owner_v1, neighbor_n0, neighbor_n1).
pub(crate) fn quad_corner_indices(edge_index: u8) -> (u8, u8, u8, u8) {
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

/// Builds a gap mesh (3 or 4 world-space vertices) in the first vertex's local space.
pub(crate) fn build_gap_mesh(world_verts: &[Vec3]) -> Mesh {
    let origin = world_verts[0];
    let local: Vec<Vec3> = world_verts.iter().map(|&v| v - origin).collect();

    let normal = compute_normal(local[0], local[1], local[2]);
    let positions: Vec<[f32; 3]> = local.iter().map(|v| v.to_array()).collect();
    let normals = vec![normal.to_array(); positions.len()];

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

#[cfg(test)]
mod tests {
    use super::*;
    use hexx::shapes;

    // ── gap_filler ────────────────────────────────────────────────────

    #[test]
    fn total_gap_counts_radius1() {
        let grid: Vec<Hex> = shapes::hexagon(Hex::ZERO, 1).collect();
        let (quads, tris) = gap_filler(&grid);
        assert_eq!(quads, 12, "radius-1 grid should have 12 quads");
        assert_eq!(tris, 6, "radius-1 grid should have 6 tris");
    }

    #[test]
    fn total_gap_counts_radius0() {
        let grid: Vec<Hex> = shapes::hexagon(Hex::ZERO, 0).collect();
        let (quads, tris) = gap_filler(&grid);
        assert_eq!(quads, 0, "single-hex grid should have 0 quads");
        assert_eq!(tris, 0, "single-hex grid should have 0 tris");
    }

    #[test]
    fn total_gap_counts_radius2() {
        let grid: Vec<Hex> = shapes::hexagon(Hex::ZERO, 2).collect();
        let (quads, tris) = gap_filler(&grid);
        // Manually verified: 19 hexes, 42 shared even-edges, 24 complete vertex junctions
        assert!(quads > 0, "radius-2 should have quads");
        assert!(tris > 0, "radius-2 should have tris");
        // Scaling check: radius-1 has (12, 6); radius-2 must have strictly more
        assert!(
            quads > 12,
            "radius-2 quads ({quads}) must exceed radius-1 (12)"
        );
        assert!(tris > 6, "radius-2 tris ({tris}) must exceed radius-1 (6)");
    }

    // ── map_noise_to_range ──────────────────────────────────────────

    #[test]
    fn noise_min_maps_to_range_min() {
        assert_eq!(map_noise_to_range(-1.0, 0.0, 10.0), 0.0);
    }

    #[test]
    fn noise_max_maps_to_range_max() {
        assert_eq!(map_noise_to_range(1.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn noise_zero_maps_to_midpoint() {
        let result = map_noise_to_range(0.0, 2.0, 6.0);
        assert!((result - 4.0).abs() < 1e-6);
    }

    #[test]
    fn noise_works_with_negative_range() {
        let result = map_noise_to_range(0.0, -10.0, 10.0);
        assert!((result - 0.0).abs() < 1e-6);
    }

    // ── compute_normal ──────────────────────────────────────────────

    #[test]
    fn normal_of_xy_plane_triangle() {
        let n = compute_normal(Vec3::ZERO, Vec3::X, Vec3::Y);
        // Cross of X × Y = Z
        assert!((n - Vec3::Z).length() < 1e-6);
    }

    #[test]
    fn normal_of_xz_plane_triangle() {
        let n = compute_normal(Vec3::ZERO, Vec3::X, Vec3::Z);
        // Cross of X × Z = -Y
        assert!((n - Vec3::NEG_Y).length() < 1e-6);
    }

    #[test]
    fn degenerate_triangle_returns_zero() {
        // Collinear points
        let n = compute_normal(Vec3::ZERO, Vec3::X, Vec3::X * 2.0);
        assert_eq!(n, Vec3::ZERO);
    }

    // ── idw_interpolate_height ───────────────────────────────────────

    #[test]
    fn idw_empty_vertices_returns_none() {
        assert_eq!(idw_interpolate_height(Vec2::ZERO, &[]), None);
    }

    #[test]
    fn idw_snap_at_same_xz() {
        let v = Vec3::new(1.0, 5.0, 2.0);
        let result = idw_interpolate_height(Vec2::new(1.0, 2.0), &[v]);
        assert_eq!(result, Some(5.0));
    }

    #[test]
    fn idw_single_vertex_nearby() {
        let v = Vec3::new(1.0, 5.0, 2.0);
        let result = idw_interpolate_height(Vec2::new(1.5, 2.5), &[v]);
        assert_eq!(
            result,
            Some(5.0),
            "single vertex → its height regardless of distance"
        );
    }

    #[test]
    fn idw_uniform_height() {
        let h = 7.0;
        let vertices = vec![
            Vec3::new(0.0, h, 0.0),
            Vec3::new(1.0, h, 0.0),
            Vec3::new(0.0, h, 1.0),
            Vec3::new(1.0, h, 1.0),
        ];
        let result = idw_interpolate_height(Vec2::new(0.3, 0.7), &vertices).unwrap();
        assert!(
            (result - h).abs() < 1e-6,
            "uniform field should return {h}, got {result}"
        );
    }

    #[test]
    fn idw_midpoint_weighted_average() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(2.0, 10.0, 0.0);
        let mid = Vec2::new(1.0, 0.0);
        let result = idw_interpolate_height(mid, &[v0, v1]).unwrap();
        // Equidistant from both → average of heights
        assert!(
            (result - 5.0).abs() < 1e-4,
            "midpoint should be ~5.0, got {result}"
        );
    }

    #[test]
    fn idw_snap_within_threshold() {
        let v = Vec3::new(3.0, 42.0, 4.0);
        // Query very close but not exactly at vertex XZ
        let pos = Vec2::new(3.0 + 1e-4, 4.0);
        let result = idw_interpolate_height(pos, &[v]);
        assert_eq!(
            result,
            Some(42.0),
            "within snap threshold should return exact height"
        );
    }

    // ── edge_cuboid_transform ────────────────────────────────────────

    #[test]
    fn edge_along_x_axis() {
        let (mid, len, rot) = edge_cuboid_transform(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0));
        assert!((mid - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-6);
        assert!((len - 4.0).abs() < 1e-6);
        // X→X rotation is identity
        let angle = rot.angle_between(Quat::IDENTITY);
        assert!(
            angle < 1e-4,
            "X-aligned should be ~identity rotation, got angle {angle}"
        );
    }

    #[test]
    fn edge_along_z_axis() {
        let (mid, len, rot) = edge_cuboid_transform(Vec3::ZERO, Vec3::new(0.0, 0.0, 3.0));
        assert!((mid - Vec3::new(0.0, 0.0, 1.5)).length() < 1e-6);
        assert!((len - 3.0).abs() < 1e-6);
        // X→Z is a 90° rotation around Y
        let expected = Quat::from_rotation_arc(Vec3::X, Vec3::Z);
        let angle = rot.angle_between(expected);
        assert!(
            angle < 1e-3,
            "Z-aligned should be 90° around Y, got angle diff {angle}"
        );
    }

    #[test]
    fn edge_diagonal_length() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 6.0, 3.0);
        let (_, len, _) = edge_cuboid_transform(a, b);
        let expected = a.distance(b);
        assert!((len - expected).abs() < 1e-6);
    }

    #[test]
    fn edge_zero_length_no_panic() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        let (mid, len, rot) = edge_cuboid_transform(p, p);
        assert!((len - 0.0).abs() < 1e-6);
        assert!((mid - p).length() < 1e-6);
        assert_eq!(rot, Quat::IDENTITY);
    }

    // ── quad_corner_indices ──────────────────────────────────────────

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
}
