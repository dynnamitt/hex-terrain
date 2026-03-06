//! Pure computation helpers extracted for testability.
//!
//! All functions in this module are free of Bevy ECS dependencies and operate
//! on plain numeric / `Vec3` inputs, making them straightforward to unit-test.

use bevy::prelude::Vec3;
use hexx::{EdgeDirection, GridVertex, Hex, VertexDirection};

/// Maps a noise value from the standard `[-1, 1]` range into `[min, max]`.
///
/// Noise generators (e.g. `Fbm<Perlin>`) produce values centred around zero.
/// This linearly rescales to an arbitrary output range.
///
/// # Examples
/// ```
/// # use hex_terrain::math::map_noise_to_range;
/// assert_eq!(map_noise_to_range(-1.0, 0.0, 10.0), 0.0);
/// assert_eq!(map_noise_to_range( 1.0, 0.0, 10.0), 10.0);
/// assert_eq!(map_noise_to_range( 0.0, 2.0, 6.0),  4.0);
/// ```
pub fn map_noise_to_range(noise_val: f64, min: f32, max: f32) -> f32 {
    min + ((noise_val as f32 + 1.0) / 2.0) * (max - min)
}

/// Cubic ease-out curve: fast start, gentle deceleration.
///
/// `t` should be in `[0, 1]`. Returns `1 - (1 - t)^3`.
///
/// Commonly used for camera animations and reveal effects.
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Computes the face normal of a triangle defined by three vertices.
///
/// Uses the cross product of edges `(v1 - v0)` and `(v2 - v0)`.
/// Returns `Vec3::ZERO` if the triangle is degenerate (collinear points).
pub fn compute_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    edge1.cross(edge2).normalize_or_zero()
}

/// Clamps a pitch angle so the camera cannot flip past vertical.
///
/// `current` is the existing pitch in radians (from `Quat::to_euler`).
/// `delta` is the desired change. The result is clamped to
/// `(-PI/2 + margin, PI/2 - margin)` and the *effective* delta is returned
/// (i.e. how much to actually rotate).
pub fn clamp_pitch(current: f32, delta: f32, margin: f32) -> f32 {
    let limit = std::f32::consts::FRAC_PI_2 - margin;
    let clamped = (current + delta).clamp(-limit, limit);
    clamped - current
}

/// Count total (quads, tris) for a grid using the same ownership rules
/// as `generate_h_grid`: quads on even edges [0,2,4] where neighbor exists,
/// tris on vertices [0,1] with canonical ownership and all 3 coords in grid.
pub fn gap_filler(grid: &[Hex]) -> (usize, usize) {
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

    // ── ease_out_cubic ──────────────────────────────────────────────

    #[test]
    fn ease_at_zero_is_zero() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
    }

    #[test]
    fn ease_at_one_is_one() {
        assert_eq!(ease_out_cubic(1.0), 1.0);
    }

    #[test]
    fn ease_at_half_is_above_half() {
        // Ease-out should be ahead of linear at the midpoint.
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn ease_is_monotonically_increasing() {
        let steps: Vec<f32> = (0..=100)
            .map(|i| ease_out_cubic(i as f32 / 100.0))
            .collect();
        for w in steps.windows(2) {
            assert!(w[1] >= w[0], "ease_out_cubic must be non-decreasing");
        }
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

    // ── clamp_pitch ─────────────────────────────────────────────────

    #[test]
    fn small_delta_passes_through() {
        let delta = clamp_pitch(0.0, 0.1, 0.05);
        assert!((delta - 0.1).abs() < 1e-6);
    }

    #[test]
    fn clamps_at_upper_limit() {
        let limit = std::f32::consts::FRAC_PI_2 - 0.05;
        // Already near limit, trying to push past
        let delta = clamp_pitch(limit - 0.01, 0.1, 0.05);
        assert!(
            (delta - 0.01).abs() < 1e-4,
            "should clamp to remaining room"
        );
    }

    #[test]
    fn clamps_at_lower_limit() {
        let limit = -(std::f32::consts::FRAC_PI_2 - 0.05);
        let delta = clamp_pitch(limit + 0.01, -0.1, 0.05);
        assert!((delta - (-0.01)).abs() < 1e-4);
    }
}
