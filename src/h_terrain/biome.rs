//! Biome: configurable mineral distribution for terrain generation.
//!
//! A [`Biome`] restricts which minerals appear and at what relative weight,
//! replacing the hardcoded scarcity-based selection in [`Mineral::from_hex`].

use hexx::Hex;

use super::mineral::{Mineral, hash_hex};

/// Weighted mineral entry in a biome.
#[derive(Clone, Debug)]
struct Entry {
    mineral: Mineral,
    weight: f32,
}

/// Biome controlling mineral distribution across the terrain.
///
/// Wraps a subset of [`Mineral`] variants with relative weights.
/// Use [`Biome::pick`] for deterministic per-hex selection.
#[derive(Clone, Debug)]
pub struct Biome {
    entries: Vec<Entry>,
    total: f32,
}

impl Biome {
    /// Create a biome from `(Mineral, weight)` pairs.
    ///
    /// Weights are relative — `[(Granite, 1.0), (Sandstone, 1.0)]` means 50/50.
    /// Panics if `entries` is empty.
    pub fn new(entries: &[(Mineral, f32)]) -> Self {
        assert!(!entries.is_empty(), "biome must have at least one mineral");
        let entries: Vec<Entry> = entries
            .iter()
            .map(|&(mineral, weight)| Entry { mineral, weight })
            .collect();
        let total = entries.iter().map(|e| e.weight).sum();
        Self { entries, total }
    }

    /// Deterministic mineral selection for a hex coordinate.
    pub fn pick(&self, hex: Hex, seed: u32) -> Mineral {
        let h = hash_hex(hex, seed);
        let val = (h % 10000) as f32 / 10000.0 * self.total;
        let mut cum = 0.0;
        for e in &self.entries {
            cum += e.weight;
            if val < cum {
                return e.mineral;
            }
        }
        self.entries.last().unwrap().mineral
    }

    /// Flora frequency for a given mineral in this biome.
    ///
    /// Delegates to [`Mineral::flora_freq`] — the biome controls *which*
    /// minerals appear, each mineral carries its own flora propensity.
    #[allow(dead_code)] // pub API for flora spawn integration
    pub fn flora_freq(&self, mineral: Mineral) -> f32 {
        mineral.flora_freq()
    }

    /// Iterate over minerals present in this biome.
    #[allow(dead_code)] // pub API for flora spawn integration
    pub fn minerals(&self) -> impl Iterator<Item = Mineral> + '_ {
        self.entries.iter().map(|e| e.mineral)
    }
}

/// Default biome: all 8 minerals at their original scarcity weights.
impl Default for Biome {
    fn default() -> Self {
        Self::new(&Mineral::ALL.map(|m| (m, m.scarcity())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_from_hex() {
        let biome = Biome::default();
        // Default biome should produce the same result as Mineral::from_hex
        for i in 0..100 {
            let hex = Hex::new(i, i * 3);
            let seed = 7919;
            assert_eq!(
                biome.pick(hex, seed),
                Mineral::from_hex(hex, seed),
                "mismatch at hex ({i}, {})",
                i * 3
            );
        }
    }

    #[test]
    fn restricted_biome_only_picks_listed() {
        let biome = Biome::new(&[(Mineral::Granite, 1.0), (Mineral::Sandstone, 1.0)]);
        for i in 0..200 {
            let m = biome.pick(Hex::new(i, -i), 42);
            assert!(
                m == Mineral::Granite || m == Mineral::Sandstone,
                "unexpected mineral {m:?} at hex ({i}, {})",
                -i
            );
        }
    }

    #[test]
    fn single_mineral_biome() {
        let biome = Biome::new(&[(Mineral::Obsidian, 1.0)]);
        for i in 0..50 {
            assert_eq!(biome.pick(Hex::new(i, 0), 99), Mineral::Obsidian);
        }
    }

    #[test]
    fn flora_freq_delegates() {
        let biome = Biome::default();
        assert_eq!(biome.flora_freq(Mineral::Sandstone), 0.05);
        assert_eq!(biome.flora_freq(Mineral::Granite), 0.0);
    }
}
