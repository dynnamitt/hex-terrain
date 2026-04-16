//! FoV shader overlay material extensions.
//!
//! Two material types share the same uniform layout (bindings 100/101):
//! - [`FovMaterial`] — hex faces: FoV tint + aim star (`aiming_overlay.wgsl`)
//! - [`BubbleFovMaterial`] — gap faces: bubble dissolve (`fov_bubbles.wgsl`)

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Stamps out a `MaterialExtension` struct with uniform bindings 100/101.
///
/// - `data.x` = fov_progress, `data.y` = aim_mode, `data.z` = shape_type, `data.w` = rotate_pace
/// - `aim_params`: x = radius, y = inner_cut, z = thickness, w = reserved
macro_rules! fov_overlay {
    ($name:ident, $shader:expr) => {
        #[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
        pub struct $name {
            #[uniform(100)]
            pub data: Vec4,
            #[uniform(101)]
            pub aim_params: Vec4,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    data: Vec4::ZERO,
                    aim_params: Vec4::ZERO,
                }
            }
        }

        impl MaterialExtension for $name {
            fn fragment_shader() -> ShaderRef {
                $shader.into()
            }

            fn deferred_fragment_shader() -> ShaderRef {
                $shader.into()
            }
        }
    };
}

fov_overlay!(FovOverlay, "shaders/aiming_overlay.wgsl");
fov_overlay!(BubbleFovOverlay, "shaders/fov_bubbles.wgsl");

/// Hex face material: `StandardMaterial` + FoV tint + aim star.
pub type FovMaterial = ExtendedMaterial<StandardMaterial, FovOverlay>;

/// Gap face material: `StandardMaterial` + bubble dissolve.
pub type BubbleFovMaterial = ExtendedMaterial<StandardMaterial, BubbleFovOverlay>;
