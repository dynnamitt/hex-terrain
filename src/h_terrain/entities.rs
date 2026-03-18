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

pub(crate) mod gap_marks {
    use bevy::prelude::*;

    /// Gap mesh entity accessor, implemented by both owner and emitter markers.
    pub(crate) trait Mark {
        /// The gap mesh entity this marker references (not a child of this corner's parent).
        fn not_owned_by_parent(&self) -> Entity;
    }
    /// Implemented by emitter markers that reference a non-owned gap mesh.
    pub(crate) trait EmitterMark {
        /// Mesh vertex index this emitter contributes to.
        fn vertex_index(&self) -> u8;
    }

    // ── Quad gap markers ─────────────────────────────────────────────

    /// Corner that owns a quad gap mesh (vertex 0 of the quad).
    /// Holds the owned mesh entity and the neighbor HCell across the edge.
    #[derive(Component, Reflect)]
    pub struct QuadOwner {
        /// The owned Quad mesh entity (child of this corner).
        pub gap: Entity,
        /// The single neighbor HCell across the edge.
        pub neighbor_hex: Entity,
    }

    /// Neighbor corner contributing vertex 1 of a quad gap.
    /// Holds the [`Quad`] mesh entity this corner aids.
    #[derive(Component, Reflect)]
    pub struct QuadPos1Emitter(pub Entity);

    /// Neighbor corner contributing vertex 2 of a quad gap.
    /// Holds the [`Quad`] mesh entity this corner aids.
    #[derive(Component, Reflect)]
    pub struct QuadPos2Emitter(pub Entity);

    /// Corner at vertex 3 of a quad gap (corner i+1 on the owning hex).
    #[derive(Component, Reflect)]
    pub struct QuadTail;

    // ── Tri gap markers ──────────────────────────────────────────────

    /// Corner that owns a tri gap mesh (vertex 0 of the triangle).
    /// Holds the owned mesh entity and the two neighbor HCell entities.
    #[derive(Component, Reflect)]
    pub struct TriOwner {
        /// The owned Tri mesh entity (child of this corner).
        pub gap: Entity,
        /// HCell entity holding the TriPos1Emitter corner.
        pub neighbor1_hex: Entity,
        /// HCell entity holding the TriPos2Emitter corner.
        pub neighbor2_hex: Entity,
    }

    /// Neighbor corner contributing vertex 1 of a tri gap.
    /// Holds the [`Tri`] mesh entity this corner aids.
    #[derive(Component, Reflect)]
    pub struct TriPos1Emitter(pub Entity);

    /// Neighbor corner contributing vertex 2 of a tri gap.
    /// Holds the [`Tri`] mesh entity this corner aids.
    #[derive(Component, Reflect)]
    pub struct TriPos2Emitter(pub Entity);

    impl EmitterMark for QuadPos1Emitter {
        fn vertex_index(&self) -> u8 {
            1
        }
    }

    impl EmitterMark for QuadPos2Emitter {
        fn vertex_index(&self) -> u8 {
            2
        }
    }

    impl EmitterMark for TriPos1Emitter {
        fn vertex_index(&self) -> u8 {
            1
        }
    }

    impl EmitterMark for TriPos2Emitter {
        fn vertex_index(&self) -> u8 {
            2
        }
    }

    impl Mark for QuadOwner {
        fn not_owned_by_parent(&self) -> Entity {
            self.gap
        }
    }

    impl Mark for TriOwner {
        fn not_owned_by_parent(&self) -> Entity {
            self.gap
        }
    }

    impl Mark for QuadPos1Emitter {
        fn not_owned_by_parent(&self) -> Entity {
            self.0
        }
    }

    impl Mark for QuadPos2Emitter {
        fn not_owned_by_parent(&self) -> Entity {
            self.0
        }
    }

    impl Mark for TriPos1Emitter {
        fn not_owned_by_parent(&self) -> Entity {
            self.0
        }
    }

    impl Mark for TriPos2Emitter {
        fn not_owned_by_parent(&self) -> Entity {
            self.0
        }
    }
}
pub(crate) use gap_marks::*;

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

/// Marker on aim-star line entities spawned on the [`InSight`] hex face.
#[derive(Component, Reflect)]
pub struct AimStar;
