use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use hexx::Hex;

use super::TerrainConfig;
use super::terrain_hex_layout::TerrainHexLayout;

/// Central component holding the terrain hex layout data.
///
/// Spawned as a single entity that parents all [`HexSunDisc`] entities.
#[derive(Component)]
pub struct HexGrid {
    /// Encapsulated hex layout with heights, radii, and vertex computation.
    pub terrain: TerrainHexLayout,
}

/// Marker for height-indicator stem entities.
#[derive(Component, Reflect)]
pub struct Stem;

/// Marker on hex face entities.
#[derive(Component, Reflect)]
pub struct HexSunDisc {
    /// The hex coordinate this disc represents.
    pub hex: Hex,
}

/// Gap quad between two adjacent hexes. Child of the owning `HexSunDisc`.
#[derive(Component, Reflect)]
pub struct QuadPetal {
    /// Even edge index on the owner hex (0, 2, or 4).
    pub edge_index: u8,
    /// Entity of the neighbor `HexSunDisc`.
    pub neighbor_disc: Entity,
}

/// Gap triangle at a 3-hex vertex junction. Child of the owning `HexSunDisc`.
#[derive(Component, Reflect)]
pub struct TriPetal {
    /// Vertex index on the owner hex (0 or 1).
    pub vertex_index: u8,
    /// The other two `HexSunDisc` entities at this junction.
    pub neighbor_discs: [Entity; 2],
}

/// Edge cuboid mesh. Child of a `QuadPetal`.
#[derive(Component, Reflect)]
pub struct QuadLines;

/// Maps hex coordinates to their spawned `HexSunDisc` entity IDs.
#[derive(Resource)]
pub struct HexEntities {
    /// Lookup from hex to entity.
    pub map: HashMap<Hex, Entity>,
}

/// Per-hex reveal state, attached to each [`HexSunDisc`].
///
/// Transitions: `Naked` → `Revealed` (in reveal ring) → `PlayerAbove` (player enters).
/// `PlayerAbove` demotes back to `Revealed` when the player leaves.
#[derive(Component, Default, Reflect, Clone)]
pub enum FlowerState {
    /// No petals spawned yet.
    #[default]
    Naked,
    /// Petals spawned; player is elsewhere.
    Revealed { petals: Vec<Entity> },
    /// Petals spawned; player is directly above this hex.
    PlayerAbove { petals: Vec<Entity> },
}

impl FlowerState {
    /// True when petal geometry has not been spawned yet.
    pub fn needs_petals(&self) -> bool {
        match self {
            Self::Naked => true,
            Self::PlayerAbove { petals } => petals.is_empty(),
            Self::Revealed { .. } => false,
        }
    }

    /// Demote `PlayerAbove` → `Revealed`, keeping existing petals.
    pub fn demote(&mut self) {
        if let Self::PlayerAbove { petals } = self {
            let petals = std::mem::take(petals);
            *self = Self::Revealed { petals };
        }
    }

    /// Promote any state → `PlayerAbove`, keeping existing petals.
    pub fn promote(&mut self) {
        match self {
            Self::Naked => *self = Self::PlayerAbove { petals: vec![] },
            Self::Revealed { petals } => {
                let petals = std::mem::take(petals);
                *self = Self::PlayerAbove { petals };
            }
            Self::PlayerAbove { .. } => {}
        }
    }

    /// Fill petals on a state that `needs_petals()`.
    /// `Naked` → `Revealed`, empty `PlayerAbove` → `PlayerAbove` with petals.
    pub fn fill_petals(&mut self, new: Vec<Entity>) {
        match self {
            Self::Naked => *self = Self::Revealed { petals: new },
            Self::PlayerAbove { petals } if petals.is_empty() => *petals = new,
            _ => {}
        }
    }
}

/// Configurable set of which edges and vertices to spawn petals for.
pub struct PetalSet {
    /// Even edge indices to spawn quad petals (e.g. `[0, 2, 4]`).
    pub quad_edges: &'static [u8],
    /// Vertex indices to spawn tri petals (e.g. `[0, 1]`).
    pub tri_vertices: &'static [u8],
}

/// All petals owned by a hex: edges 0, 2, 4 and vertices 0, 1.
pub const FULL_PETAL_SET: PetalSet = PetalSet {
    quad_edges: &[0, 2, 4],
    tri_vertices: &[0, 1],
};

/// Per-hex iteration data passed to petal spawn helpers.
pub struct HexCtx {
    pub(super) hex: Hex,
    pub(super) owner_entity: Entity,
    pub(super) inverse_tf: Transform,
}

/// Shared material handles for the neon visual theme (used by petal spawning).
#[derive(Resource)]
pub struct NeonMaterials {
    /// Bright emissive cyan used for edge lines.
    pub edge_material: Handle<StandardMaterial>,
    /// Slightly warm dark material for gap-fill quads and triangles.
    pub gap_face_material: Handle<StandardMaterial>,
}

/// Bundled read-only system parameters for petal spawning.
#[derive(SystemParam)]
pub struct PetalRes<'w, 's> {
    /// The hex grid component query.
    pub grid_q: Query<'w, 's, &'static HexGrid>,
    /// Hex coordinate → entity mapping.
    pub hex_entities: Res<'w, HexEntities>,
    /// Shared neon material handles.
    pub neon: Res<'w, NeonMaterials>,
    /// Terrain configuration.
    pub cfg: Res<'w, TerrainConfig>,
}

/// Shared immutable context passed to petal spawn helpers.
pub struct PetalCtx<'a> {
    /// Hex entity lookup.
    pub hex_entities: &'a HexEntities,
    /// Material handles.
    pub neon: &'a NeonMaterials,
    /// Grid data (layout, heights, vertices).
    pub grid: &'a HexGrid,
    /// Terrain config (edge thickness, etc.).
    pub cfg: &'a TerrainConfig,
}
