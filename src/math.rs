//! Cross-module computation helpers.
//!
//! Terrain-specific math lives in `h_terrain::math`.

/// Cubic ease-out curve: fast start, gentle deceleration.
///
/// `t` should be in `[0, 1]`. Returns `1 - (1 - t)^3`.
///
/// Commonly used for camera animations and reveal effects.
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
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

#[cfg(test)]
mod tests {
    use super::*;

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
