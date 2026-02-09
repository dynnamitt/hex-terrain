//! Pure computation helpers extracted for testability.
//!
//! All functions in this module are free of Bevy ECS dependencies and operate
//! on plain numeric / `Vec3` inputs, making them straightforward to unit-test.

use bevy::prelude::Vec3;

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

/// Brightness multiplier for height-indicator poles based on camera distance.
///
/// Returns a value in `[min_alpha, 1.0]`:
/// - At `distance = 0` the pole is dimmest (`min_alpha`).
/// - At `distance >= fade_distance` the pole is fully bright (`1.0`).
///
/// The intent is to fade poles that are directly under the camera so they
/// don't obscure the terrain.
pub fn pole_fade_brightness(distance: f32, fade_distance: f32, min_alpha: f32) -> f32 {
    let t = (distance / fade_distance).clamp(0.0, 1.0);
    // Inverted: close = dim, far = bright
    min_alpha + t * (1.0 - min_alpha)
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

/// Geometry parameters for a height-indicator pole.
#[derive(Debug, PartialEq)]
pub struct PoleGeometry {
    /// World-space radius of the cylinder.
    pub radius: f32,
    /// Total height of the cylinder.
    pub height: f32,
    /// Y coordinate of the cylinder centre (half the height).
    pub y_center: f32,
}

/// Computes pole cylinder dimensions from a hex's visual radius and face height.
///
/// Returns `None` when the face is at or below ground level (no pole needed).
/// `radius_factor` controls how thick the pole is relative to the hex,
/// and `gap` leaves a small space between pole top and hex face.
pub fn pole_geometry(
    hex_radius: f32,
    face_height: f32,
    radius_factor: f32,
    gap: f32,
) -> Option<PoleGeometry> {
    let pole_height = face_height - gap;
    if pole_height <= 0.0 {
        return None;
    }
    Some(PoleGeometry {
        radius: hex_radius * radius_factor,
        height: pole_height,
        y_center: pole_height / 2.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // ── pole_fade_brightness ────────────────────────────────────────

    #[test]
    fn at_zero_distance_returns_min_alpha() {
        let b = pole_fade_brightness(0.0, 40.0, 0.05);
        assert!((b - 0.05).abs() < 1e-6);
    }

    #[test]
    fn at_fade_distance_returns_one() {
        let b = pole_fade_brightness(40.0, 40.0, 0.05);
        assert!((b - 1.0).abs() < 1e-6);
    }

    #[test]
    fn beyond_fade_distance_clamps_to_one() {
        let b = pole_fade_brightness(100.0, 40.0, 0.05);
        assert!((b - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mid_distance_is_between_min_and_one() {
        let b = pole_fade_brightness(20.0, 40.0, 0.05);
        assert!(b > 0.05 && b < 1.0);
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

    // ── pole_geometry ───────────────────────────────────────────────

    #[test]
    fn pole_for_elevated_hex() {
        let pg = pole_geometry(1.0, 5.0, 0.06, 0.05).unwrap();
        assert!((pg.radius - 0.06).abs() < 1e-6);
        assert!((pg.height - 4.95).abs() < 1e-6);
        assert!((pg.y_center - 4.95 / 2.0).abs() < 1e-6);
    }

    #[test]
    fn pole_at_ground_level_returns_none() {
        assert!(pole_geometry(1.0, 0.05, 0.06, 0.05).is_none());
    }

    #[test]
    fn pole_below_ground_returns_none() {
        assert!(pole_geometry(1.0, -1.0, 0.06, 0.05).is_none());
    }
}
