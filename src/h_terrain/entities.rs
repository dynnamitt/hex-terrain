//! Entity types for height-based terrain.

use bevy::platform::collections::HashMap;
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
    /// Hex → HCell entity lookup.
    pub hex_entities: HashMap<Hex, Entity>,
}

/// Marker on [`HCell`] entities within the player's field-of-view radius.
#[derive(Component, Reflect)]
pub struct InFov;

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
/// Holds the [`Quad`] mesh entity this corner aids.
#[derive(Component, Reflect)]
pub struct QuadPos2Emitter(pub Entity);

/// Neighbor corner contributing vertex 2 of a quad gap.
/// Holds the [`Quad`] mesh entity this corner aids.
#[derive(Component, Reflect)]
pub struct QuadPos3Emitter(pub Entity);

/// Corner at vertex 3 of a quad gap (corner i+1 on the owning hex).
#[derive(Component, Reflect)]
pub struct QuadTail;

// ── Tri gap markers ──────────────────────────────────────────────

/// Corner that owns a tri gap mesh (vertex 0 of the triangle).
#[derive(Component, Reflect)]
pub struct TriOwner;

/// Neighbor corner contributing vertex 1 of a tri gap.
/// Holds the [`Tri`] mesh entity this corner aids.
#[derive(Component, Reflect)]
pub struct TriPos1Emitter(pub Entity);

/// Neighbor corner contributing vertex 2 of a tri gap.
/// Holds the [`Tri`] mesh entity this corner aids.
#[derive(Component, Reflect)]
pub struct TriPos2Emitter(pub Entity);

// ── Mesh entity markers ─────────────────────────────────────────

/// Marker on quad gap mesh entities (child of a [`QuadOwner`] corner).
#[derive(Component, Reflect)]
pub struct Quad;

/// Marker on tri gap mesh entities (child of a [`TriOwner`] corner).
#[derive(Component, Reflect)]
pub struct Tri;

/// Marker on hex face mesh entities (child of [`HCell`]).
#[derive(Component, Reflect)]
pub struct HexFace;

/// Marker on edge-line cuboid entities (child of a [`Quad`]).
#[derive(Component, Reflect)]
pub struct QuadEdge;

/// Tracks an in-progress color transition between original and highlight materials.
#[derive(Component, Reflect)]
pub struct FovTransition {
    /// 0.0 = original colors, 1.0 = highlight colors.
    pub progress: f32,
    /// +1.0 when fading toward highlight, -1.0 when fading toward original.
    pub direction: f32,
}

/// Marker on the single hex face the camera is looking directly at.
#[derive(Component, Reflect)]
pub struct InSight;

/// Stashed material handle from before [`InSight`] was applied.
#[derive(Component, Reflect)]
pub struct PreSightMaterial(pub Handle<StandardMaterial>);

/// Material handles for [`InFov`] highlighting.
#[derive(Resource)]
pub struct FovMaterials {
    /// Original hex face material.
    pub hex_original: Handle<StandardMaterial>,
    /// Highlight hex face material (emissive warm glow).
    pub hex_highlight: Handle<StandardMaterial>,
    /// Original gap (Quad/Tri) material.
    pub gap_original: Handle<StandardMaterial>,
    /// Highlight gap material (emissive cyan glow).
    pub gap_highlight: Handle<StandardMaterial>,
    /// Purple emissive material for the aimed-at hex face (screen center + within FoV).
    pub hex_in_aim: Handle<StandardMaterial>,
}
