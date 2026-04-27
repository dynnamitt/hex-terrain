//! Hex grid layout: spatial mapping, noise-driven heights/radii, vertex computation.

use std::collections::HashMap;

use glam::{Vec2, Vec3};
use hexx::{EdgeDirection, GridVertex, Hex, HexLayout, VertexDirection, shapes};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::math;

/// Grid layout and noise parameters.
#[derive(Clone, Debug)]
pub struct HGridSettings {
    /// Number of hex rings around the origin (~1200 hexes at 20).
    pub radius: u32,
    /// Distance in world-units between adjacent hex centers.
    pub point_spacing: f32,
    /// Seed for the height noise generator.
    pub height_noise_seed: u32,
    /// Seed for the per-hex radius noise generator.
    pub radius_noise_seed: u32,
    /// Number of octaves for height noise.
    pub height_noise_octaves: usize,
    /// Number of octaves for radius noise.
    pub radius_noise_octaves: usize,
    /// Spatial scale divisor for height noise sampling.
    pub height_noise_scale: f64,
    /// Spatial scale divisor for radius noise sampling.
    pub radius_noise_scale: f64,
    /// Maximum terrain elevation produced by the noise function.
    pub max_height: f32,
    /// Smallest visual hex radius (noise-derived per cell).
    pub min_hex_radius: f32,
    /// Largest visual hex radius (noise-derived per cell).
    pub max_hex_radius: f32,
}

impl Default for HGridSettings {
    fn default() -> Self {
        Self {
            radius: 20,
            point_spacing: 4.0,
            height_noise_seed: 43,
            radius_noise_seed: 137,
            height_noise_octaves: 4,
            radius_noise_octaves: 3,
            height_noise_scale: 50.0,
            radius_noise_scale: 30.0,
            max_height: 20.0,
            min_hex_radius: 0.2,
            max_hex_radius: 2.6,
        }
    }
}

/// Encapsulates the hex layout, per-cell heights/radii, and vertex computation.
///
/// Owns only the data needed for grid generation and height interpolation —
/// no ECS dependencies.
pub struct HGridLayout {
    layout: HexLayout,
    unit_corners: [Vec2; 6],
    grid_radius: u32,
    heights: HashMap<Hex, f32>,
    radii: HashMap<Hex, f32>,
}

impl HGridLayout {
    /// Constructs the layout from grid settings, sampling noise for heights and radii.
    pub fn from_settings(g: &HGridSettings) -> Self {
        let layout = HexLayout {
            scale: Vec2::splat(g.point_spacing),
            ..HexLayout::default()
        };
        let unit_layout = HexLayout {
            scale: Vec2::splat(1.0),
            ..HexLayout::default()
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
            grid_radius: g.radius,
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

    /// Returns the two long-side edges of each quad gap as `(from, to)` pairs
    /// in world-space XZ (projected, Y=0). Uses the even-edge `[0,2,4]` ownership
    /// rule so each gap is emitted exactly once — no overlaps.
    ///
    /// "Long sides" are the edges that bridge hex→neighbor (v0→n0 and v1→n1),
    /// not the short edges running along each hex's perimeter.
    pub fn quad_long_edges(&self) -> Vec<(Vec3, Vec3)> {
        let hexes: Vec<Hex> = shapes::hexagon(Hex::ZERO, self.grid_radius).collect();
        let mut edges = Vec::new();
        for &hex in &hexes {
            for edge_index in [0u8, 2, 4] {
                let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
                let neighbor = hex.neighbor(dir);
                if !self.heights.contains_key(&neighbor) {
                    continue;
                }
                let (v0_idx, v1_idx, n0_idx, n1_idx) = math::quad_corner_indices(edge_index);
                if let (Some(a), Some(b), Some(c), Some(d)) = (
                    self.vertex(hex, v0_idx),
                    self.vertex(neighbor, n0_idx),
                    self.vertex(hex, v1_idx),
                    self.vertex(neighbor, n1_idx),
                ) {
                    edges.push((a, b));
                    edges.push((c, d));
                }
            }
        }
        edges
    }

    /// Returns the four world-space corners `[hex.v0, neighbor.n0, neighbor.n1, hex.v1]`
    /// of every gap quad, in CCW order ready for triangulation `[0,1,2,0,2,3]`.
    /// Same `[0,2,4]` even-edge ownership as [`quad_long_edges`].
    pub fn gap_quads(&self) -> Vec<[Vec3; 4]> {
        let hexes: Vec<Hex> = shapes::hexagon(Hex::ZERO, self.grid_radius).collect();
        let mut quads = Vec::new();
        for &hex in &hexes {
            for edge_index in [0u8, 2, 4] {
                let dir = EdgeDirection::ALL_DIRECTIONS[edge_index as usize];
                let neighbor = hex.neighbor(dir);
                if !self.heights.contains_key(&neighbor) {
                    continue;
                }
                let (v0_idx, v1_idx, n0_idx, n1_idx) = math::quad_corner_indices(edge_index);
                if let (Some(a), Some(b), Some(c), Some(d)) = (
                    self.vertex(hex, v0_idx),
                    self.vertex(neighbor, n0_idx),
                    self.vertex(neighbor, n1_idx),
                    self.vertex(hex, v1_idx),
                ) {
                    quads.push([a, b, c, d]);
                }
            }
        }
        quads
    }

    /// Returns the six center-fan triangles of every hex's flat top face,
    /// in world-space. Each hex contributes 6 tris from the centroid to
    /// consecutive corner pairs (CCW). Hard-coded fan — no earcut needed
    /// for the flat-top case.
    pub fn hex_face_tris(&self) -> Vec<[Vec3; 3]> {
        let hexes: Vec<Hex> = shapes::hexagon(Hex::ZERO, self.grid_radius).collect();
        let mut tris = Vec::new();
        for &hex in &hexes {
            let Some(&height) = self.heights.get(&hex) else {
                continue;
            };
            let center2 = self.layout.hex_to_world_pos(hex);
            let center = Vec3::new(center2.x, height, center2.y);
            for i in 0..6u8 {
                let j = (i + 1) % 6;
                if let (Some(a), Some(b)) = (self.vertex(hex, i), self.vertex(hex, j)) {
                    tris.push([center, a, b]);
                }
            }
        }
        tris
    }

    /// Unified triangle stream: gap quads (split along the rust-canonical
    /// `[v0, v2]` diagonal), gap junction tris, and hex face fan tris.
    /// All tessellation decisions owned here so clients consume one flat
    /// list with no per-source branching.
    pub fn all_tris(&self) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        for [a, b, c, d] in self.gap_quads() {
            out.push([a, b, c]);
            out.push([a, c, d]);
        }
        out.extend(self.gap_tris());
        out.extend(self.hex_face_tris());
        out
    }

    /// Returns the three world-space corners of every 3-hex junction gap tri.
    /// Canonical ownership: tri emitted only when the origin hex is `coordinates()[0]`.
    pub fn gap_tris(&self) -> Vec<[Vec3; 3]> {
        let hexes: Vec<Hex> = shapes::hexagon(Hex::ZERO, self.grid_radius).collect();
        let mut tris = Vec::new();
        for &hex in &hexes {
            for v_idx in [0u8, 1] {
                let dir = VertexDirection::ALL_DIRECTIONS[v_idx as usize];
                let gv = GridVertex {
                    origin: hex,
                    direction: dir,
                };
                let coords = gv.coordinates();
                if coords[0] != hex {
                    continue;
                }
                if !coords.iter().all(|c| self.heights.contains_key(c)) {
                    continue;
                }
                let idx1 = corner_index_for_vertex(coords[1], &gv);
                let idx2 = corner_index_for_vertex(coords[2], &gv);
                if let (Some(i1), Some(i2)) = (idx1, idx2)
                    && let (Some(v0), Some(v1), Some(v2)) = (
                        self.vertex(coords[0], v_idx),
                        self.vertex(coords[1], i1),
                        self.vertex(coords[2], i2),
                    )
                {
                    tris.push([v0, v1, v2]);
                }
            }
        }
        tris
    }

    /// Grid radius (number of hex rings around the origin).
    pub fn grid_radius(&self) -> u32 {
        self.grid_radius
    }

    /// World-units between adjacent hex centers (the layout `scale.x`).
    pub fn point_spacing(&self) -> f32 {
        self.layout.scale.x
    }

    /// Inverse-distance-weighted height interpolation from nearby hex vertices.
    pub fn interpolate_height(&self, pos: Vec2) -> f32 {
        let hex = self.layout.world_pos_to_hex(pos);
        let vertices: Vec<Vec3> = std::iter::once(hex)
            .chain(hex.all_neighbors())
            .flat_map(|h| (0..6u8).filter_map(move |i| self.vertex(h, i)))
            .collect();
        math::idw_interpolate_height(pos, &vertices)
            .unwrap_or_else(|| self.heights.get(&hex).copied().unwrap_or(0.0))
    }
}

fn corner_index_for_vertex(hex: Hex, target: &GridVertex) -> Option<u8> {
    VertexDirection::ALL_DIRECTIONS.iter().find_map(|&dir| {
        let candidate = GridVertex {
            origin: hex,
            direction: dir,
        };
        candidate.equivalent(target).then_some(dir.index())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_settings() -> HGridSettings {
        HGridSettings::default()
    }

    #[test]
    fn from_settings_populates_all_hexes() {
        let g = default_settings();
        let layout = HGridLayout::from_settings(&g);
        let expected = shapes::hexagon(Hex::ZERO, g.radius).count();
        assert_eq!(layout.heights.len(), expected);
        assert_eq!(layout.radii.len(), expected);
    }

    #[test]
    fn hex_to_world_and_back_roundtrip() {
        let g = default_settings();
        let layout = HGridLayout::from_settings(&g);
        for hex in shapes::hexagon(Hex::ZERO, 3) {
            let world = layout.hex_to_world_pos(hex);
            let back = layout.world_pos_to_hex(world);
            assert_eq!(hex, back, "roundtrip failed for {hex:?}");
        }
    }

    #[test]
    fn vertex_returns_six_positions_per_hex() {
        let g = default_settings();
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
            ..default_settings()
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
        let g = default_settings();
        let layout = HGridLayout::from_settings(&g);
        let corners: Vec<Vec2> = (0..6).map(|i| layout.unit_corner(i)).collect();
        for i in 0..6 {
            for j in (i + 1)..6 {
                assert_ne!(corners[i], corners[j], "corners {i} and {j} are identical");
            }
        }
    }

    #[test]
    fn hex_face_tris_count_is_six_per_hex() {
        for r in [0u32, 1, 2, 4] {
            let g = HGridSettings {
                radius: r,
                ..default_settings()
            };
            let layout = HGridLayout::from_settings(&g);
            let hex_count = shapes::hexagon(Hex::ZERO, r).count();
            let tris = layout.hex_face_tris();
            assert_eq!(
                tris.len(),
                hex_count * 6,
                "radius {r}: expected {} fan tris, got {}",
                hex_count * 6,
                tris.len()
            );
        }
    }

    #[test]
    fn hex_face_tris_share_height_per_hex() {
        let g = HGridSettings {
            radius: 1,
            ..default_settings()
        };
        let layout = HGridLayout::from_settings(&g);
        let tris = layout.hex_face_tris();
        for (i, tri) in tris.iter().enumerate() {
            let y0 = tri[0].y;
            let y1 = tri[1].y;
            let y2 = tri[2].y;
            assert!(
                (y0 - y1).abs() < 1e-4 && (y0 - y2).abs() < 1e-4,
                "tri {i}: heights differ ({y0}, {y1}, {y2}) — face should be flat"
            );
        }
    }

    #[test]
    fn all_tris_count_matches_components() {
        for r in [1u32, 2, 4] {
            let g = HGridSettings {
                radius: r,
                ..default_settings()
            };
            let layout = HGridLayout::from_settings(&g);
            let quads = layout.gap_quads().len();
            let gap_tris = layout.gap_tris().len();
            let face_tris = layout.hex_face_tris().len();
            let total = layout.all_tris().len();
            assert_eq!(
                total,
                quads * 2 + gap_tris + face_tris,
                "radius {r}: all_tris should equal quads*2 + gap_tris + face_tris"
            );
        }
    }

    #[test]
    fn quad_long_edges_count_matches_gap_filler() {
        for r in [1, 2, 4] {
            let g = HGridSettings {
                radius: r,
                ..default_settings()
            };
            let layout = HGridLayout::from_settings(&g);
            let grid: Vec<Hex> = shapes::hexagon(Hex::ZERO, r).collect();
            let (expected_quads, _) = crate::math::gap_filler(&grid);
            let edges = layout.quad_long_edges();
            assert_eq!(
                edges.len(),
                expected_quads * 2,
                "radius {r}: expected {} long edges ({}×2), got {}",
                expected_quads * 2,
                expected_quads,
                edges.len()
            );
        }
    }
}
