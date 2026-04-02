//! Re-exports [`hex_grid::HGridLayout`] and bridges from h_terrain's [`HGridSettings`].

use super::HGridSettings;

pub use hex_grid::HGridLayout;

impl HGridSettings {
    /// Build an [`HGridLayout`] from these settings.
    pub fn build_layout(&self) -> HGridLayout {
        HGridLayout::from_settings(&self.to_grid_settings())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::h_terrain::HTerrainConfig;
    use bevy::prelude::Vec2;
    use hexx::{Hex, shapes};

    fn default_grid_settings() -> HGridSettings {
        HTerrainConfig::default().grid
    }

    #[test]
    fn from_settings_populates_all_hexes() {
        let g = default_grid_settings();
        let layout = g.build_layout();
        let expected = shapes::hexagon(Hex::ZERO, g.radius).count();
        // Verify all hexes have heights via the public API.
        let count = shapes::hexagon(Hex::ZERO, g.radius)
            .filter(|h| layout.height(h).is_some())
            .count();
        assert_eq!(count, expected);
    }

    #[test]
    fn hex_to_world_and_back_roundtrip() {
        let g = default_grid_settings();
        let layout = g.build_layout();
        for hex in shapes::hexagon(Hex::ZERO, 3) {
            let world = layout.hex_to_world_pos(hex);
            let back = layout.world_pos_to_hex(world);
            assert_eq!(hex, back, "roundtrip failed for {hex:?}");
        }
    }

    #[test]
    fn vertex_returns_six_positions_per_hex() {
        let g = default_grid_settings();
        let layout = g.build_layout();
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
        let layout = g.build_layout();
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
        let layout = g.build_layout();
        let corners: Vec<Vec2> = (0..6).map(|i| layout.unit_corner(i)).collect();
        for i in 0..6 {
            for j in (i + 1)..6 {
                assert_ne!(corners[i], corners[j], "corners {i} and {j} are identical");
            }
        }
    }
}
