use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use hexx::{Hex, HexLayout, VertexDirection, shapes};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::math;
use crate::terrain::GridSettings;

/// Encapsulates the hex layout, per-cell heights/radii, and vertex computation.
///
/// Vertices are computed on demand from `layout + unit_corners + height + radius`
/// rather than stored in a HashMap.
pub struct TerrainHexLayout {
    layout: HexLayout,
    unit_corners: [Vec2; 6],
    heights: HashMap<Hex, f32>,
    radii: HashMap<Hex, f32>,
}

impl TerrainHexLayout {
    /// Constructs the layout from grid/flower settings, sampling noise for heights and radii.
    pub fn from_settings(g: &GridSettings) -> Self {
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

    /// Minimal constructor for tests: a single hex with explicit height and radius.
    #[cfg(test)]
    pub fn single(hex: Hex, height: f32, radius: f32, spacing: f32) -> Self {
        let layout = HexLayout {
            scale: Vec2::splat(spacing),
            ..default()
        };
        let unit_layout = HexLayout {
            scale: Vec2::splat(1.0),
            ..default()
        };
        let unit_corners: [Vec2; 6] =
            std::array::from_fn(|i| unit_layout.center_aligned_hex_corners()[i]);

        let mut heights = HashMap::new();
        let mut radii = HashMap::new();
        heights.insert(hex, height);
        radii.insert(hex, radius);

        Self {
            layout,
            unit_corners,
            heights,
            radii,
        }
    }

    // ── Coordinate conversion ──────────────────────────────────────

    /// World-space 2D position of a hex center (delegates to inner HexLayout).
    pub fn hex_to_world_pos(&self, hex: Hex) -> Vec2 {
        self.layout.hex_to_world_pos(hex)
    }

    /// Hex coordinate from a world-space 2D position (delegates to inner HexLayout).
    pub fn world_pos_to_hex(&self, pos: Vec2) -> Hex {
        self.layout.world_pos_to_hex(pos)
    }

    // ── Per-hex data access ────────────────────────────────────────

    /// Whether this hex exists in the grid.
    pub fn contains(&self, hex: &Hex) -> bool {
        self.heights.contains_key(hex)
    }

    /// Noise-derived terrain height for a hex.
    pub fn height(&self, hex: &Hex) -> Option<f32> {
        self.heights.get(hex).copied()
    }

    /// Noise-derived visual radius for a hex.
    pub fn radius(&self, hex: &Hex) -> Option<f32> {
        self.radii.get(hex).copied()
    }

    /// Computed world-space vertex position for `hex` at corner `index` (0..5).
    ///
    /// `center + unit_corners[index] * radius` at the hex's height.
    pub fn vertex(&self, hex: Hex, index: u8) -> Option<Vec3> {
        let &height = self.heights.get(&hex)?;
        let &radius = self.radii.get(&hex)?;
        let center = self.layout.hex_to_world_pos(hex);
        let offset = self.unit_corners[index as usize] * radius;
        Some(Vec3::new(center.x + offset.x, height, center.y + offset.y))
    }

    // ── Compute methods ────────────────────────────────────────────

    /// Inverse-distance-weighted height interpolation from nearby hex vertices.
    pub fn interpolate_height(&self, pos: Vec2) -> f32 {
        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;

        let hex = self.layout.world_pos_to_hex(pos);
        let hexes_to_check: Vec<Hex> = std::iter::once(hex).chain(hex.all_neighbors()).collect();

        for h in hexes_to_check {
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

    /// Inverse transform that cancels the parent HexSunDisc's translation + scale.
    pub fn inverse_transform(&self, hex: Hex) -> Transform {
        let center_2d = self.layout.hex_to_world_pos(hex);
        let height = self.heights[&hex];
        let radius = self.radii[&hex];

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

    /// Finds the vertex position on `hex` that corresponds to the same grid vertex as `target`.
    pub fn find_equivalent_vertex(&self, hex: Hex, target: &hexx::GridVertex) -> Option<Vec3> {
        for dir in VertexDirection::ALL_DIRECTIONS {
            let candidate = hexx::GridVertex {
                origin: hex,
                direction: dir,
            };
            if candidate.equivalent(target) {
                return self.vertex(hex, dir.index());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::TerrainConfig;

    fn default_grid_settings() -> GridSettings {
        TerrainConfig::default().grid
    }

    #[test]
    fn from_settings_populates_all_hexes() {
        let g = default_grid_settings();
        let terrain = TerrainHexLayout::from_settings(&g);
        let expected = shapes::hexagon(Hex::ZERO, g.radius).count();
        assert_eq!(terrain.heights.len(), expected);
        assert_eq!(terrain.radii.len(), expected);
    }

    #[test]
    fn hex_to_world_and_back_roundtrip() {
        let g = default_grid_settings();
        let terrain = TerrainHexLayout::from_settings(&g);
        for hex in shapes::hexagon(Hex::ZERO, 3) {
            let world = terrain.hex_to_world_pos(hex);
            let back = terrain.world_pos_to_hex(world);
            assert_eq!(hex, back, "roundtrip failed for {hex:?}");
        }
    }

    #[test]
    fn contains_returns_false_for_out_of_bounds() {
        let terrain = TerrainHexLayout::single(Hex::ZERO, 5.0, 1.0, 4.0);
        assert!(terrain.contains(&Hex::ZERO));
        assert!(!terrain.contains(&Hex::new(100, 100)));
    }

    #[test]
    fn height_and_radius_return_none_for_missing() {
        let terrain = TerrainHexLayout::single(Hex::ZERO, 5.0, 1.0, 4.0);
        let far = Hex::new(99, 99);
        assert!(terrain.height(&far).is_none());
        assert!(terrain.radius(&far).is_none());
    }

    #[test]
    fn vertex_returns_six_positions_per_hex() {
        let terrain = TerrainHexLayout::single(Hex::ZERO, 3.0, 1.5, 4.0);
        for i in 0..6u8 {
            assert!(
                terrain.vertex(Hex::ZERO, i).is_some(),
                "vertex {i} should exist"
            );
        }
    }

    #[test]
    fn vertex_position_matches_manual_computation() {
        let hex = Hex::ZERO;
        let height = 7.0;
        let radius = 2.0;
        let spacing = 4.0;
        let terrain = TerrainHexLayout::single(hex, height, radius, spacing);

        let center = terrain.hex_to_world_pos(hex);

        for i in 0..6u8 {
            let v = terrain.vertex(hex, i).unwrap();
            let expected_offset = terrain.unit_corners[i as usize] * radius;
            let expected = Vec3::new(
                center.x + expected_offset.x,
                height,
                center.y + expected_offset.y,
            );
            assert!(
                (v - expected).length() < 1e-5,
                "vertex {i}: got {v:?}, expected {expected:?}"
            );
        }
    }

    #[test]
    fn interpolate_at_center_uniform() {
        let terrain = TerrainHexLayout::single(Hex::ZERO, 3.0, 1.0, 4.0);
        let h = terrain.interpolate_height(Vec2::ZERO);
        assert!(
            (h - 3.0).abs() < 0.1,
            "uniform height should be ~3.0, got {h}"
        );
    }

    #[test]
    fn interpolate_at_vertex_near_exact() {
        let terrain = TerrainHexLayout::single(Hex::ZERO, 5.0, 1.0, 4.0);
        let vpos = terrain.vertex(Hex::ZERO, 0).unwrap();
        let pos = Vec2::new(vpos.x + 0.0001, vpos.z + 0.0001);
        let h = terrain.interpolate_height(pos);
        assert!(
            (h - 5.0).abs() < 0.1,
            "height near vertex should be ~5.0, got {h}"
        );
    }

    #[test]
    fn interpolate_outside_grid_fallback() {
        let terrain = TerrainHexLayout::single(Hex::ZERO, 7.0, 1.0, 4.0);
        let h = terrain.interpolate_height(Vec2::new(1000.0, 1000.0));
        assert!(h >= 0.0);
    }

    #[test]
    fn inverse_transform_cancels_parent() {
        let hex = Hex::ZERO;
        let height = 5.0;
        let radius = 2.0;
        let terrain = TerrainHexLayout::single(hex, height, radius, 4.0);

        let center = terrain.hex_to_world_pos(hex);
        let parent_tf = Transform::from_xyz(center.x, height, center.y)
            .with_scale(Vec3::new(radius, 1.0, radius));
        let inv = terrain.inverse_transform(hex);

        let combined = parent_tf.mul_transform(inv);
        assert!(
            combined.translation.length() < 1e-4,
            "combined translation should be near zero, got {:?}",
            combined.translation
        );
        assert!(
            (combined.scale - Vec3::ONE).length() < 1e-4,
            "combined scale should be near one, got {:?}",
            combined.scale
        );
    }
}
