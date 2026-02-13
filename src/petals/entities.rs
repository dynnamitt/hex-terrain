use bevy::ecs::system::SystemParam;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use hexx::Hex;

use super::PetalsConfig;
use crate::AppConfig;
use crate::grid::HexGrid;
use crate::visuals::ActiveNeonMaterials;

/// Marker for height-indicator pole entities. Spawned by `grid.rs`.
#[derive(Component, Reflect)]
pub struct HeightPole;

/// Marker on hex face entities. Spawned by `grid.rs`.
#[derive(Component, Reflect)]
pub struct HexSunDisc {
    /// The hex coordinate this disc represents.
    pub hex: Hex,
}

/// Gap quad between two adjacent hexes. Child of the owning `HexSunDisc`.
#[derive(Component, Reflect)]
pub struct QuadLeaf {
    /// Even edge index on the owner hex (0, 2, or 4).
    pub edge_index: u8,
    /// Entity of the neighbor `HexSunDisc`.
    pub neighbor_disc: Entity,
}

/// Gap triangle at a 3-hex vertex junction. Child of the owning `HexSunDisc`.
#[derive(Component, Reflect)]
pub struct TriLeaf {
    /// Vertex index on the owner hex (0 or 1).
    pub vertex_index: u8,
    /// The other two `HexSunDisc` entities at this junction.
    pub neighbor_discs: [Entity; 2],
}

/// Edge cuboid mesh. Child of a `QuadLeaf`.
#[derive(Component, Reflect)]
pub struct PetalEdge;

/// Maps hex coordinates to their spawned `HexSunDisc` entity IDs.
#[derive(Resource)]
pub struct HexEntities {
    /// Lookup from hex to entity.
    pub map: HashMap<Hex, Entity>,
}

/// Tracks which hexes have already had their petals spawned.
#[derive(Resource, Default)]
pub struct DrawnCells {
    pub(super) cells: HashSet<Hex>,
}

// ── Shared context bundles ──────────────────────────────────────────

/// Read-only resources needed by all petal spawn helpers.
#[derive(SystemParam)]
pub struct PetalRes<'w, 's> {
    pub(super) grid_q: Query<'w, 's, &'static HexGrid>,
    pub(super) hex_entities: Res<'w, HexEntities>,
    pub(super) neon: Res<'w, ActiveNeonMaterials>,
    pub(super) config: Res<'w, AppConfig>,
    pub(super) petals_cfg: Res<'w, PetalsConfig>,
}

/// Per-hex iteration data passed to leaf spawn helpers.
pub struct HexCtx {
    pub(super) hex: Hex,
    pub(super) owner_entity: Entity,
    pub(super) inverse_tf: Transform,
}
