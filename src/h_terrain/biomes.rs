//! Named biome presets built from the current mineral set.
#![allow(dead_code)] // presets are pub API, wired via CLI / config

use super::biome::Biome;
use super::mineral::Mineral;

/// All minerals at their natural scarcity weights.
pub fn standard() -> Biome {
    Biome::new(&Mineral::ALL.map(|m| (m, m.scarcity())))
}

/// Rocky highlands: dark igneous stone dominates, rare crystal veins.
pub fn rocky() -> Biome {
    Biome::new(&[
        (Mineral::Granite, 30.0),
        (Mineral::Basalt, 25.0),
        (Mineral::Slate, 20.0),
        (Mineral::Obsidian, 10.0),
        (Mineral::Quartz, 2.0),
    ])
}

/// Arid mesa: warm sandstone and marble with copper seams.
pub fn mesa() -> Biome {
    Biome::new(&[
        (Mineral::Sandstone, 35.0),
        (Mineral::Marble, 20.0),
        (Mineral::Granite, 15.0),
        (Mineral::Copper, 8.0),
        (Mineral::Quartz, 5.0),
    ])
}

/// Lush mesa: sand-heavy variant with more flora-eligible terrain.
pub fn mesa_lush() -> Biome {
    Biome::new(&[
        (Mineral::Sandstone, 50.0),
        (Mineral::Marble, 15.0),
        (Mineral::Granite, 10.0),
        (Mineral::Copper, 6.0),
        (Mineral::Quartz, 3.0),
    ])
}

#[cfg(test)]
mod tests {
    use hexx::Hex;

    use super::*;

    fn covers_only(biome: &Biome, expected: &[Mineral], seed: u32) {
        let minerals: std::collections::HashSet<Mineral> = (0..500)
            .map(|i| biome.pick(Hex::new(i, i * 3), seed))
            .collect();
        for &m in expected {
            assert!(minerals.contains(&m), "{m:?} missing from biome output");
        }
        for &m in &minerals {
            assert!(expected.contains(&m), "unexpected {m:?} in biome output");
        }
    }

    #[test]
    fn standard_covers_all() {
        covers_only(&standard(), &Mineral::ALL, 7919);
    }

    #[test]
    fn rocky_subset() {
        covers_only(
            &rocky(),
            &[
                Mineral::Granite,
                Mineral::Basalt,
                Mineral::Slate,
                Mineral::Obsidian,
                Mineral::Quartz,
            ],
            42,
        );
    }

    #[test]
    fn mesa_subset() {
        covers_only(
            &mesa(),
            &[
                Mineral::Sandstone,
                Mineral::Marble,
                Mineral::Granite,
                Mineral::Copper,
                Mineral::Quartz,
            ],
            42,
        );
    }

    #[test]
    fn mesa_lush_subset() {
        covers_only(
            &mesa_lush(),
            &[
                Mineral::Sandstone,
                Mineral::Marble,
                Mineral::Granite,
                Mineral::Copper,
                Mineral::Quartz,
            ],
            42,
        );
    }
}
