//! Procedural flora meshes and cluster spawning for hex-terrain.
//!
//! Provides hexagonal shimeji mushroom geometry (6-sided cross-section,
//! bent stem with variable diameter, domed cap) and a cluster spawn
//! function that places 1–3 mushrooms using shared mesh handles.

pub mod cluster;
pub mod mesh;

use bevy::prelude::*;

// ── Marker components ──────────────────────────────────────────────

/// Individual mushroom root (parent of stem + cap).
#[derive(Component)]
pub struct Shimeji;

/// Stem mesh entity.
#[derive(Component)]
pub struct ShimejiStem;

/// Cap mesh entity.
#[derive(Component)]
pub struct ShimejiCap;

// ── Configuration ──────────────────────────────────────────────────

/// Geometry config for shimeji mushrooms. Defaults match the Blender model.
#[derive(Clone, Debug)]
pub struct FloraCfg {
    /// Hex cross-section sides.
    pub sides: u32,
    /// Full stem height (tallest mushroom).
    pub stem_height: f32,
    /// Stem base radius (widest).
    pub stem_base_r: f32,
    /// Stem narrowest radius (~40% up).
    pub stem_narrow_r: f32,
    /// Stem top radius (cap junction).
    pub stem_top_r: f32,
    /// Lateral bend magnitude X.
    pub stem_bend_x: f32,
    /// Lateral bend magnitude Z (Bevy coords).
    pub stem_bend_z: f32,
    /// Total twist along stem (degrees).
    pub stem_twist_deg: f32,
    /// Number of vertical stem segments.
    pub stem_seg: u32,
    /// Cap outer radius.
    pub cap_r: f32,
    /// Cap dome height.
    pub cap_h: f32,
    /// Number of concentric cap rings.
    pub cap_rings: u32,
    /// Clone position offset in XZ plane.
    pub clone_offset: Vec3,
    /// Clone Y-rotation (degrees).
    pub clone_rot_y_deg: f32,
    /// Clone vertical scale factor (stem shortening).
    pub clone_scale_y: f32,
}

impl Default for FloraCfg {
    fn default() -> Self {
        Self {
            sides: 6,
            stem_height: 1.5,
            stem_base_r: 0.14,
            stem_narrow_r: 0.08,
            stem_top_r: 0.11,
            stem_bend_x: 0.12,
            stem_bend_z: 0.06,
            stem_twist_deg: 15.0,
            stem_seg: 10,
            cap_r: 0.32,
            cap_h: 0.20,
            cap_rings: 3,
            clone_offset: Vec3::new(0.3, 0.0, 0.18),
            clone_rot_y_deg: 140.0,
            clone_scale_y: 0.7,
        }
    }
}

// ── Shared asset handles ───────────────────────────────────────────

/// Shared mesh + material handles for all shimeji instances.
#[derive(Resource)]
pub struct FloraMaterials {
    /// Cream stem material.
    pub stem_mat: Handle<StandardMaterial>,
    /// Tan-brown cap material.
    pub cap_mat: Handle<StandardMaterial>,
    /// Shared stem mesh (one for all instances).
    pub stem_mesh: Handle<Mesh>,
    /// Shared cap mesh (one for all instances).
    pub cap_mesh: Handle<Mesh>,
}

impl FloraMaterials {
    /// Build materials and meshes from config.
    pub fn new(
        mats: &mut Assets<StandardMaterial>,
        meshes: &mut Assets<Mesh>,
        cfg: &FloraCfg,
    ) -> Self {
        Self {
            stem_mat: mats.add(StandardMaterial {
                base_color: Color::srgb(0.92, 0.88, 0.78),
                perceptual_roughness: 0.8,
                ..default()
            }),
            cap_mat: mats.add(StandardMaterial {
                base_color: Color::srgb(0.55, 0.40, 0.28),
                perceptual_roughness: 0.7,
                ..default()
            }),
            stem_mesh: meshes.add(mesh::build_stem(cfg)),
            cap_mesh: meshes.add(mesh::build_cap(cfg)),
        }
    }
}
