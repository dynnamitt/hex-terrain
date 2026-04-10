//! FoV shader overlay material extension.
//!
//! Wraps `StandardMaterial` with a `FovOverlay` extension that drives FoV highlight,
//! aim, and fire effects through uniforms — no material handle swapping needed.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Shader overlay data for face entities (HexFace, Quad, Tri).
///
/// - `data.x` = `fov_progress` (0.0..1.0): FoV highlight intensity
/// - `data.y` = `aim_mode` (0 = none, 1 = aim, 2 = firing)
/// - `data.z` = `shape_type` (0 = hex, 1 = quad, 2 = tri)
/// - `data.w` = reserved
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct FovOverlay {
    #[uniform(100)]
    pub data: Vec4,
}

impl Default for FovOverlay {
    fn default() -> Self {
        Self { data: Vec4::ZERO }
    }
}

impl MaterialExtension for FovOverlay {
    fn fragment_shader() -> ShaderRef {
        "shaders/fov_overlay.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/fov_overlay.wgsl".into()
    }
}

/// Face material type: `StandardMaterial` + `FovOverlay` extension.
pub type FovMaterial = ExtendedMaterial<StandardMaterial, FovOverlay>;
