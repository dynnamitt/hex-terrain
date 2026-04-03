//! Mineral types with per-variant visual properties and scarcity weights.

use bevy::color::Mix;
use bevy::prelude::*;
use hexx::Hex;

/// Per-HCell terrain mineral, driving material appearance and scarcity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum Mineral {
    Granite,
    Basalt,
    Slate,
    Sandstone,
    Obsidian,
    Marble,
    Quartz,
    Copper,
}

struct Props {
    color: [f32; 3],
    #[expect(dead_code, reason = "retained for future normal-mapped geometry")]
    roughness: f32,
    #[expect(dead_code, reason = "retained for future normal-mapped geometry")]
    metallic: f32,
    scarcity: f32,
}

const PROPS: [Props; Mineral::COUNT] = [
    Props {
        color: [0.45, 0.42, 0.38],
        roughness: 0.8,
        metallic: 0.1,
        scarcity: 30.0,
    },
    Props {
        color: [0.25, 0.27, 0.30],
        roughness: 0.85,
        metallic: 0.05,
        scarcity: 25.0,
    },
    Props {
        color: [0.35, 0.38, 0.45],
        roughness: 0.6,
        metallic: 0.15,
        scarcity: 20.0,
    },
    Props {
        color: [0.60, 0.50, 0.30],
        roughness: 0.9,
        metallic: 0.0,
        scarcity: 20.0,
    },
    Props {
        color: [0.10, 0.10, 0.12],
        roughness: 0.15,
        metallic: 0.5,
        scarcity: 5.0,
    },
    Props {
        color: [0.75, 0.73, 0.70],
        roughness: 0.3,
        metallic: 0.1,
        scarcity: 5.0,
    },
    Props {
        color: [0.65, 0.55, 0.60],
        roughness: 0.4,
        metallic: 0.3,
        scarcity: 3.0,
    },
    Props {
        color: [0.60, 0.35, 0.15],
        roughness: 0.5,
        metallic: 0.7,
        scarcity: 2.0,
    },
];

/// Mix factor for FoV highlight (base_color toward white).
pub(crate) const HIGHLIGHT_MIX: f32 = 0.15;

/// Tiny emissive glow applied to FoV-highlighted hex faces and gaps.
pub const HIGHLIGHT_EMISSIVE: LinearRgba = LinearRgba::new(0.03, 0.03, 0.03, 1.0);

impl Mineral {
    pub const ALL: [Self; 8] = [
        Self::Granite,
        Self::Basalt,
        Self::Slate,
        Self::Sandstone,
        Self::Obsidian,
        Self::Marble,
        Self::Quartz,
        Self::Copper,
    ];
    pub const COUNT: usize = Self::ALL.len();

    pub fn idx(self) -> usize {
        self as usize
    }

    fn props(self) -> &'static Props {
        &PROPS[self.idx()]
    }

    pub fn scarcity(self) -> f32 {
        self.props().scarcity
    }

    pub fn color(self) -> Color {
        let [r, g, b] = self.props().color;
        Color::srgb(r, g, b)
    }

    pub fn material(self) -> StandardMaterial {
        StandardMaterial {
            base_color: self.color(),
            perceptual_roughness: 0.5,
            metallic: 0.0,
            cull_mode: None,
            ..default()
        }
    }

    /// FoV highlight: base color mixed toward white.
    pub fn highlight_color(self) -> Color {
        let o = LinearRgba::from(self.color());
        Color::from(o.mix(&LinearRgba::WHITE, HIGHLIGHT_MIX))
    }

    pub fn highlight_material(self) -> StandardMaterial {
        StandardMaterial {
            base_color: self.highlight_color(),
            perceptual_roughness: 0.5,
            metallic: 0.0,
            emissive: HIGHLIGHT_EMISSIVE,
            cull_mode: None,
            ..default()
        }
    }

    /// Deterministic mineral selection from hex coordinates + seed.
    pub fn from_hex(hex: Hex, seed: u32) -> Self {
        let h = hash_hex(hex, seed);
        let total: f32 = Self::ALL.iter().map(|m| m.scarcity()).sum();
        let val = (h % 10000) as f32 / 10000.0 * total;
        let mut cum = 0.0;
        for &m in &Self::ALL {
            cum += m.scarcity();
            if val < cum {
                return m;
            }
        }
        *Self::ALL.last().unwrap()
    }
}

fn hash_hex(hex: Hex, seed: u32) -> u32 {
    let mut h = (hex.x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((hex.y as u32).wrapping_mul(668265263))
        .wrapping_add(seed);
    h ^= h >> 13;
    h = h.wrapping_mul(1274126177);
    h ^= h >> 16;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_indexed() {
        for (i, &m) in Mineral::ALL.iter().enumerate() {
            assert_eq!(m.idx(), i);
        }
    }

    #[test]
    fn scarcity_positive() {
        for &m in &Mineral::ALL {
            assert!(m.scarcity() > 0.0);
        }
    }

    #[test]
    fn from_hex_deterministic() {
        let a = Mineral::from_hex(Hex::ZERO, 42);
        let b = Mineral::from_hex(Hex::ZERO, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn from_hex_produces_variety() {
        let minerals: Vec<Mineral> = (0..50)
            .map(|i| Mineral::from_hex(Hex::new(i, 0), 42))
            .collect();
        let unique: std::collections::HashSet<_> = minerals.into_iter().collect();
        assert!(unique.len() > 1, "should produce multiple mineral types");
    }

    #[test]
    fn highlight_at_least_as_bright() {
        for &m in &Mineral::ALL {
            let orig = LinearRgba::from(m.color());
            let hi = LinearRgba::from(m.highlight_color());
            assert!(
                hi.red + hi.green + hi.blue >= orig.red + orig.green + orig.blue - 1e-6,
                "{m:?}: highlight should be at least as bright as original"
            );
        }
    }
}
