#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

// FovOverlay uniform: x=fov_progress, y=aim_mode (0/1/2), z=shape_type (0/1/2), w=reserved
struct FovOverlayData {
    data: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> fov_overlay: FovOverlayData;

// Band intensity pattern: 0001112232211000 (16 steps, symmetric)
const BANDS: array<f32, 16> = array<f32, 16>(
    0.0, 0.0, 0.0, 1.0,
    1.0, 1.0, 2.0, 2,
    3.0, 2.0, 2.0, 1.0,
    1.0, 0.0, 0.0, 0.0,
);

// Flat-top hexagonal distance (L∞ hex norm) from UV center
fn hex_band(uv: vec2<f32>) -> f32 {
    let p = abs(uv - vec2<f32>(0.5, 0.5));
    let d = max(p.y * 2.0 / sqrt(3.0), p.x + p.y / sqrt(3.0)) / 0.5;
    let idx = clamp(u32(d * 16.0), 0u, 15u);
    return BANDS[idx] / 3.0;
}

// Chebyshev distance — rectangular contours matching quad shape
fn quad_band(uv: vec2<f32>) -> f32 {
    let p = abs(uv - vec2<f32>(0.5, 0.5));
    let d = max(p.x, p.y) / 0.5;
    let idx = clamp(u32(d * 16.0), 0u, 15u);
    return BANDS[idx] / 3.0;
}

// Barycentric minimum — triangular contours for UV layout [0,0],[1,0],[0.5,1]
fn tri_band(uv: vec2<f32>) -> f32 {
    let l0 = 1.0 - uv.x - 0.5 * uv.y;
    let l1 = uv.x - 0.5 * uv.y;
    let l2 = uv.y;
    let d = 1.0 - min(l0, min(l1, l2)) * 3.0;
    let idx = clamp(u32(d * 16.0), 0u, 15u);
    return BANDS[idx] / 3.0;
}

fn shape_band(uv: vec2<f32>, shape_type: f32) -> f32 {
    if shape_type < 0.5 { return hex_band(uv); }
    else if shape_type < 1.5 { return quad_band(uv); }
    else { return tri_band(uv); }
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let progress = fov_overlay.data.x;
    let aim_mode = fov_overlay.data.y;
    let shape_type = fov_overlay.data.z;
    let band = shape_band(in.uv, shape_type);

    // FoV overlay: white tint rings, scaled by progress
    if progress > 0.0 {
        let intensity = band * progress * 0.05;
        pbr_input.material.base_color += vec4<f32>(intensity, intensity, intensity, 0.0);
        let em = band * progress * 0.01;
        pbr_input.material.emissive += vec4<f32>(em, em, em, 0.0);
    }

    // Aim overlay: green rings (overrides FoV tint on base_color)
    if aim_mode > 0.5 && aim_mode < 1.5 {
        let intensity = band * 0.15;
        pbr_input.material.base_color += vec4<f32>(
            0.2 * intensity, 0.9 * intensity, 0.3 * intensity, 0.0
        );
        let em = band * 0.03;
        pbr_input.material.emissive += vec4<f32>(0.2 * em, 0.9 * em, 0.3 * em, 0.0);
    }

    // Fire overlay: yellow rings
    if aim_mode > 1.5 {
        let intensity = band * 0.2;
        pbr_input.material.base_color += vec4<f32>(
            1.0 * intensity, 0.85 * intensity, 0.0, 0.0
        );
        let em = band * 0.05;
        pbr_input.material.emissive += vec4<f32>(1.0 * em, 0.85 * em, 0.0, 0.0);
    }

    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color
    );

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
