//! Re-exports pure computation helpers from the [`hex_grid`] crate.

use bevy::prelude::*;
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

/// Converts world-space gap vertices to origin-local positions and a normal.
///
/// When `flat` is true, returns a fixed Y-up normal matching [`PlaneMeshBuilder`]
/// hex faces — keeps lighting consistent regardless of terrain height variation.
/// When false, computes the actual surface normal via cross product.
pub(super) fn gap_vertex_data(world_verts: &[Vec3], flat: bool) -> (Vec<[f32; 3]>, [f32; 3]) {
    let origin = world_verts[0];
    let local: Vec<Vec3> = world_verts.iter().map(|&v| v - origin).collect();
    let normal = if flat {
        Vec3::Y
    } else {
        // Negate: vertex winding produces a downward normal but gaps face up.
        let cross = (local[1] - local[0]).cross(local[2] - local[0]);
        -cross.normalize_or_zero()
    };
    let positions = local.iter().map(|v| v.to_array()).collect();
    (positions, normal.to_array())
}

/// Count total (quads, tris) for a grid using the same ownership rules
/// as `generate_h_grid`: quads on even edges [0,2,4] where neighbor exists,
/// tris on vertices [0,1] with canonical ownership and all 3 coords in grid.
pub(crate) fn gap_filler(grid: &[Hex]) -> (usize, usize) {
    let quads = grid
        .iter()
        .flat_map(|hex| [0, 2, 4].map(|i| hex.neighbor(EdgeDirection::ALL_DIRECTIONS[i])))
        .filter(|n| grid.contains(n))
        .count();

    let tris = grid
        .iter()
        .flat_map(|&hex| {
            [0usize, 1].into_iter().filter_map(move |vi| {
                let gv = GridVertex {
                    origin: hex,
                    direction: VertexDirection::ALL_DIRECTIONS[vi],
                };
                let coords = gv.coordinates();
                (coords[0] == hex && coords.iter().all(|c| grid.contains(c))).then_some(())
            })
        })
        .count();

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

    // ── gap_vertex_data ────────────────────────────────────────────────

    #[test]
    fn gap_vertex_data_flat_triangle() {
        let verts = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
        ];
        let (positions, normal) = gap_vertex_data(&verts, true);
        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0], [0.0, 0.0, 0.0], "first vertex is origin");
        assert_eq!(positions[1], [1.0, 0.0, 0.0]);
        assert_eq!(positions[2], [0.0, 0.0, 1.0]);
        assert_eq!(normal, [0.0, 1.0, 0.0], "flat mode → Y-up");
    }

    #[test]
    fn gap_vertex_data_flat_quad() {
        let verts = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let (positions, normal) = gap_vertex_data(&verts, true);
        assert_eq!(positions.len(), 4);
        assert_eq!(positions[0], [0.0, 0.0, 0.0]);
        assert_eq!(normal, [0.0, 1.0, 0.0], "flat mode → Y-up");
    }

    #[test]
    fn gap_vertex_data_tilted_flat_still_y_up() {
        let verts = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(0.5, 1.0, 1.0),
        ];
        let (_, normal) = gap_vertex_data(&verts, true);
        assert_eq!(normal, [0.0, 1.0, 0.0], "flat mode ignores tilt");
    }

    #[test]
    fn gap_vertex_data_computed_normal() {
        let verts = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let (_, normal) = gap_vertex_data(&verts, false);
        let n = Vec3::from_array(normal);
        assert!(
            (n - Vec3::Y).length() < 1e-6,
            "flat XZ tri → Y-up even computed"
        );
    }

    #[test]
    fn gap_vertex_data_computed_tilted() {
        let verts = [
            Vec3::ZERO,
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let (_, normal) = gap_vertex_data(&verts, false);
        let n = Vec3::from_array(normal);
        assert!(n.y > 0.0, "computed normal should face up");
        assert!(n.y < 1.0, "tilted surface → non-vertical normal");
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
}
