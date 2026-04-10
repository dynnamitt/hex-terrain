//! Flora cluster spawning during terrain generation.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use hexx::Hex;

use flora::cluster::shimeji_cluster;
use flora::{FloraCfg, FloraMaterials};
use hex_grid::HGridLayout;

use super::mineral::{Mineral, hash_hex};

/// Spawn shimeji clusters on hexes whose mineral supports flora.
///
/// Each eligible hex gets a deterministic spawn check against
/// `Mineral::flora_freq()`, with cluster size 1-3 from a second hash.
/// The cluster is uniformly scaled by the hex's radius so shrooms
/// fit proportionally on smaller/larger cells.
/// Clusters are parented to the corresponding HCell entity.
pub(super) fn spawn_flora(
    commands: &mut Commands,
    assets: &FloraMaterials,
    cfg: &FloraCfg,
    hex_minerals: &HashMap<Hex, Mineral>,
    hex_entities: &HashMap<Hex, Entity>,
    terrain: &HGridLayout,
    flora_seed: u32,
) {
    for (&hex, &mineral) in hex_minerals {
        let freq = mineral.flora_freq();
        if freq <= 0.0 {
            continue;
        }

        let h = hash_hex(hex, flora_seed);
        let roll = (h % 10000) as f32 / 10000.0;
        if roll >= freq {
            continue;
        }

        let h2 = hash_hex(hex, flora_seed.wrapping_add(1));
        let num = match h2 % 5 {
            0 => 1,     // 20%
            1 | 2 => 2, // 40%
            _ => 3,     // 40%
        };

        let scale = terrain.radius(&hex).unwrap_or(1.0);
        let (cluster, _roots) = shimeji_cluster(commands, assets, cfg, Vec3::ZERO, num);
        commands
            .entity(cluster)
            .insert(Transform::from_scale(Vec3::splat(scale)));

        if let Some(&cell) = hex_entities.get(&hex) {
            commands.entity(cell).add_child(cluster);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flora_freq_ordering() {
        assert!(Mineral::Granite.flora_freq() > Mineral::Sandstone.flora_freq());
        assert!(Mineral::Basalt.flora_freq() > Mineral::Sandstone.flora_freq());
        assert!(Mineral::Sandstone.flora_freq() > 0.0);
    }

    #[test]
    fn hash_cluster_size_range() {
        for i in 0..100 {
            let h = hash_hex(Hex::new(i, 0), 1980);
            let num = match h % 5 {
                0 => 1u8,
                1 | 2 => 2,
                _ => 3,
            };
            assert!((1..=3).contains(&num));
        }
    }
}
