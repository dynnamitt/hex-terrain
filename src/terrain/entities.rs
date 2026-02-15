use bevy::ecs::system::SystemParam;
use bevy::platform::collections::{HashMap, HashSet};
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

/// Marker for height-indicator pole entities.
#[derive(Component, Reflect)]
pub struct HeightPole;

/// Marker on hex face entities.
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

/// Tracks which hex cell the player currently occupies.
#[derive(Resource, Default)]
pub struct ActiveHex {
    /// Hex coordinate directly below the player.
    pub current: Hex,
    /// The cell occupied last frame (if it moved).
    pub previous: Option<Hex>,
    /// `true` for exactly one frame after a cell transition.
    pub changed: bool,
}

/// Per-hex iteration data passed to leaf spawn helpers.
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
    /// Current hex under the player.
    pub cell: Res<'w, ActiveHex>,
}

/// Shared immutable context passed to leaf spawn helpers.
pub struct LeafCtx<'a> {
    /// Hex entity lookup.
    pub hex_entities: &'a HexEntities,
    /// Material handles.
    pub neon: &'a NeonMaterials,
    /// Grid data (layout, heights, vertices).
    pub grid: &'a HexGrid,
    /// Terrain config (edge thickness, etc.).
    pub cfg: &'a TerrainConfig,
}
