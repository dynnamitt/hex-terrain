//! Entity types for height-based terrain.

use bevy::prelude::*;
use hexx::Hex;

use super::h_grid_layout::HGridLayout;

/// Central component holding the h_terrain hex layout data.
///
/// Spawned as a single entity that parents all [`HCell`] entities.
#[derive(Component)]
pub struct HGrid {
    /// Encapsulated hex layout with heights, radii, and vertex computation.
    pub terrain: HGridLayout,
}

/// Per-hex cell entity, positioned at the hex center.
#[derive(Component, Reflect)]
pub struct HCell {
    /// The hex coordinate this cell represents.
    pub hex: Hex,
}

/// Corner pivot-point entity (child of [`HCell`]).
#[derive(Component, Reflect)]
pub struct Corner {
    /// Vertex index within the parent hex (0..5).
    pub index: u8,
}

// ── Quad gap markers ─────────────────────────────────────────────

/// Corner that owns a quad gap mesh (vertex 0 of the quad).
#[derive(Component, Reflect)]
pub struct QuadOwner;

/// Neighbor corner contributing vertex 1 of a quad gap.
#[derive(Component, Reflect)]
pub struct QuadPos2Emitter;

/// Neighbor corner contributing vertex 2 of a quad gap.
#[derive(Component, Reflect)]
pub struct QuadPos3Emitter;

/// Corner at vertex 3 of a quad gap (corner i+1 on the owning hex).
#[derive(Component, Reflect)]
pub struct QuadTail;

// ── Tri gap markers ──────────────────────────────────────────────

/// Corner that owns a tri gap mesh (vertex 0 of the triangle).
#[derive(Component, Reflect)]
pub struct TriOwner;

/// Neighbor corner contributing vertex 1 of a tri gap.
#[derive(Component, Reflect)]
pub struct TriPos1Emitter;

/// Neighbor corner contributing vertex 2 of a tri gap.
#[derive(Component, Reflect)]
pub struct TriPos2Emitter;

// ── Mesh entity markers ─────────────────────────────────────────

/// Marker on quad gap mesh entities (child of a [`QuadOwner`] corner).
#[derive(Component, Reflect)]
pub struct Quad;

/// Marker on tri gap mesh entities (child of a [`TriOwner`] corner).
#[derive(Component, Reflect)]
pub struct Tri;
