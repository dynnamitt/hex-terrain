//! FoV shader overlay material extensions.
//!
//! Two material types share the same uniform layout (bindings 100/101):
//! - [`FovMaterial`] — hex faces: FoV tint + aim star (`aiming_overlay.wgsl`)
//! - [`BubbleFovMaterial`] — gap faces: bubble dissolve (`fov_bubbles.wgsl`)

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Shader overlay for hex face entities (FoV tint + aim star).
///
/// - `data.x` = `fov_progress` (0.0..1.0)
/// - `data.y` = `aim_mode` (0 = none, 1 = aim, 2 = firing)
/// - `data.z` = `shape_type` (0 = hex)
/// - `data.w` = `aim_star_rotate_pace` (rad/s)
///
/// `aim_params` (binding 101):
/// - x = outer radius, y = inner cut, z = thickness, w = reserved
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct FovOverlay {
    #[uniform(100)]
    pub data: Vec4,
    #[uniform(101)]
    pub aim_params: Vec4,
}

impl Default for FovOverlay {
    fn default() -> Self {
        Self {
            data: Vec4::ZERO,
            aim_params: Vec4::ZERO,
        }
    }
}

impl MaterialExtension for FovOverlay {
    fn fragment_shader() -> ShaderRef {
        "shaders/aiming_overlay.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/aiming_overlay.wgsl".into()
    }
}

/// Hex face material: `StandardMaterial` + FoV tint + aim star.
pub type FovMaterial = ExtendedMaterial<StandardMaterial, FovOverlay>;

/// Shader overlay for gap face entities (bubble dissolve).
///
/// Same uniform layout as [`FovOverlay`] — `data.x` drives bubble intensity,
/// `data.z` holds shape_type (1 = quad, 2 = tri). Aim fields are unused.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct BubbleFovOverlay {
    #[uniform(100)]
    pub data: Vec4,
    #[uniform(101)]
    pub aim_params: Vec4,
}

impl Default for BubbleFovOverlay {
    fn default() -> Self {
        Self {
            data: Vec4::ZERO,
            aim_params: Vec4::ZERO,
        }
    }
}

impl MaterialExtension for BubbleFovOverlay {
    fn fragment_shader() -> ShaderRef {
        "shaders/fov_bubbles.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/fov_bubbles.wgsl".into()
    }
}

/// Gap face material: `StandardMaterial` + bubble dissolve.
pub type BubbleFovMaterial = ExtendedMaterial<StandardMaterial, BubbleFovOverlay>;
