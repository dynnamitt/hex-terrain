use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use hexx::{Hex, HexLayout, shapes};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::math;

use super::HGridSettings;

/// Encapsulates the hex layout, per-cell heights/radii, and vertex computation.
///
/// Slimmed-down layout for the h_terrain module — owns only the data needed for
/// pivot-point grid generation and height interpolation.
pub struct HGridLayout {
    layout: HexLayout,
    unit_corners: [Vec2; 6],
    heights: HashMap<Hex, f32>,
    radii: HashMap<Hex, f32>,
}

impl HGridLayout {
    /// Constructs the layout from grid settings, sampling noise for heights and radii.
    pub fn from_settings(g: &HGridSettings) -> Self {
        let layout = HexLayout {
            scale: Vec2::splat(g.point_spacing),
            ..default()
        };
        let unit_layout = HexLayout {
            scale: Vec2::splat(1.0),
            ..default()
        };
        let unit_corners_slice = unit_layout.center_aligned_hex_corners();
        let unit_corners: [Vec2; 6] = std::array::from_fn(|i| unit_corners_slice[i]);

        let height_fbm: Fbm<Perlin> =
            Fbm::new(g.height_noise_seed).set_octaves(g.height_noise_octaves);
        let radius_fbm: Fbm<Perlin> =
            Fbm::new(g.radius_noise_seed).set_octaves(g.radius_noise_octaves);

        let mut heights = HashMap::new();
        let mut radii = HashMap::new();

        for hex in shapes::hexagon(Hex::ZERO, g.radius) {
            let pos = layout.hex_to_world_pos(hex);

            let noise_val = height_fbm.get([
                pos.x as f64 / g.height_noise_scale,
                pos.y as f64 / g.height_noise_scale,
            ]);
            heights.insert(hex, math::map_noise_to_range(noise_val, 0.0, g.max_height));

            let radius_noise = radius_fbm.get([
                pos.x as f64 / g.radius_noise_scale,
                pos.y as f64 / g.radius_noise_scale,
            ]);
            radii.insert(
                hex,
                math::map_noise_to_range(radius_noise, g.min_hex_radius, g.max_hex_radius),
            );
        }

        Self {
            layout,
            unit_corners,
            heights,
            radii,
        }
    }

    // ── Coordinate conversion ──────────────────────────────────────

    /// World-space 2D position of a hex center.
    pub fn hex_to_world_pos(&self, hex: Hex) -> Vec2 {
        self.layout.hex_to_world_pos(hex)
    }

    /// Hex coordinate from a world-space 2D position.
    #[allow(dead_code)]
    pub fn world_pos_to_hex(&self, pos: Vec2) -> Hex {
        self.layout.world_pos_to_hex(pos)
    }

    // ── Per-hex data access ────────────────────────────────────────

    /// Noise-derived terrain height for a hex.
    pub fn height(&self, hex: &Hex) -> Option<f32> {
        self.heights.get(hex).copied()
    }

    /// Noise-derived visual radius for a hex.
    pub fn radius(&self, hex: &Hex) -> Option<f32> {
        self.radii.get(hex).copied()
    }

    /// Computed world-space vertex position for `hex` at corner `index` (0..5).
    pub fn vertex(&self, hex: Hex, index: u8) -> Option<Vec3> {
        let &height = self.heights.get(&hex)?;
        let &radius = self.radii.get(&hex)?;
        let center = self.layout.hex_to_world_pos(hex);
        let offset = self.unit_corners[index as usize] * radius;
        Some(Vec3::new(center.x + offset.x, height, center.y + offset.y))
    }

    /// Unit corner offset for a given corner index (0..5).
    pub fn unit_corner(&self, index: usize) -> Vec2 {
        self.unit_corners[index]
    }

    // ── Compute methods ────────────────────────────────────────────

    /// Inverse-distance-weighted height interpolation from nearby hex vertices.
    pub fn interpolate_height(&self, pos: Vec2) -> f32 {
        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;

        let hex = self.layout.world_pos_to_hex(pos);

        for h in std::iter::once(hex).chain(hex.all_neighbors()) {
            for i in 0..6u8 {
                if let Some(vpos) = self.vertex(h, i) {
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
            self.heights.get(&hex).copied().unwrap_or(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::h_terrain::HTerrainConfig;

    fn default_grid_settings() -> HGridSettings {
        HTerrainConfig::default().grid
    }

    #[test]
    fn from_settings_populates_all_hexes() {
        let g = default_grid_settings();
        let layout = HGridLayout::from_settings(&g);
        let expected = shapes::hexagon(Hex::ZERO, g.radius).count();
        assert_eq!(layout.heights.len(), expected);
        assert_eq!(layout.radii.len(), expected);
    }

    #[test]
    fn hex_to_world_and_back_roundtrip() {
        let g = default_grid_settings();
        let layout = HGridLayout::from_settings(&g);
        for hex in shapes::hexagon(Hex::ZERO, 3) {
            let world = layout.hex_to_world_pos(hex);
            let back = layout.world_pos_to_hex(world);
            assert_eq!(hex, back, "roundtrip failed for {hex:?}");
        }
    }

    #[test]
    fn vertex_returns_six_positions_per_hex() {
        let g = default_grid_settings();
        let layout = HGridLayout::from_settings(&g);
        for i in 0..6u8 {
            assert!(
                layout.vertex(Hex::ZERO, i).is_some(),
                "vertex {i} should exist"
            );
        }
    }

    #[test]
    fn interpolate_at_center_uniform_height() {
        let g = HGridSettings {
            radius: 1,
            ..default_grid_settings()
        };
        let layout = HGridLayout::from_settings(&g);
        let h = layout.interpolate_height(Vec2::ZERO);
        let center_h = layout.height(&Hex::ZERO).unwrap();
        assert!(
            (h - center_h).abs() < 2.0,
            "interpolated height {h} should be near center height {center_h}"
        );
    }

    #[test]
    fn unit_corner_returns_six_distinct_offsets() {
        let g = default_grid_settings();
        let layout = HGridLayout::from_settings(&g);
        let corners: Vec<Vec2> = (0..6).map(|i| layout.unit_corner(i)).collect();
        for i in 0..6 {
            for j in (i + 1)..6 {
                assert_ne!(corners[i], corners[j], "corners {i} and {j} are identical");
            }
        }
    }
}
