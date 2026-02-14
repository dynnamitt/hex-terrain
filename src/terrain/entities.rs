use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use hexx::Hex;

/// Central component holding the hex layout, per-cell noise data, and vertex positions.
///
/// Spawned as a single entity that parents all [`HexSunDisc`] entities.
#[derive(Component)]
pub struct HexGrid {
    /// Hex-to-world coordinate mapping (spacing, orientation).
    pub layout: hexx::HexLayout,
    /// Noise-derived terrain height for each hex cell.
    pub heights: HashMap<Hex, f32>,
    /// Noise-derived visual radius for each hex cell.
    pub radii: HashMap<Hex, f32>,
    /// World-space position of each hex vertex, keyed by `(hex, vertex_index 0..5)`.
    pub vertex_positions: HashMap<(Hex, u8), Vec3>,
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
