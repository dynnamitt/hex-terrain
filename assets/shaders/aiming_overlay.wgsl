#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_view_bindings::globals,
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

// ── Uniforms ────────────────────────────────────────────────────
// data (binding 100): x=fov_progress, y=aim_mode (0/1/2), z=shape_type (0/1/2), w=aim_star_rotate_pace
// aim_params (binding 101): x=radius, y=inner_cut, z=thickness, w=reserved
struct FovOverlayData { data: vec4<f32>, }
struct AimParamsData  { aim_params: vec4<f32>, }

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> fov_overlay: FovOverlayData;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> aim_data: AimParamsData;

// ── FoV band tuning ─────────────────────────────────────────────
const HEX_BAND_INNER: f32 = 0.1;
const HEX_BAND_PEAK: f32 = 0.45;
const HEX_BAND_FADE: f32 = 0.55;
const HEX_BAND_OUTER: f32 = 0.9;
const GAP_BAND_START: f32 = 0.2;

// ── FoV tint intensities ───────────────────────────────────────
const HEX_FOV_TINT: f32 = 0.05;
const HEX_FOV_EMISSIVE: f32 = 0.01;
const GAP_FOV_TINT: f32 = 0.06;
const GAP_FOV_CYAN: vec3<f32> = vec3<f32>(0.3, 1.0, 1.0);

// ── Gap edge darkening ─────────────────────────────────────────
const EDGE_DIM_START: f32 = 0.7;
const EDGE_DIM_STRENGTH: f32 = 1.0 / 9.0;

// ── Aim star colors ────────────────────────────────────────────
const AIM_STAR_COLOR: vec4<f32> = vec4<f32>(0.9, 0.35, 0.0, 1.0);
const FIRE_STAR_COLOR: vec4<f32> = vec4<f32>(1.0, 0.15, 0.0, 1.0);
const AIM_GLOW_RGB: vec3<f32> = vec3<f32>(0.4, 0.24, 0.0);
const GLOW_MARGIN: f32 = 0.05;

// ── Shape distance: 0 at center, 1 at edge ─────────────────────
fn shape_dist(uv: vec2<f32>, shape_type: f32) -> f32 {
    if shape_type < 0.5 {
        let p = abs(uv - vec2<f32>(0.5, 0.5));
        return max(p.y * 2.0 / sqrt(3.0), p.x + p.y / sqrt(3.0)) / 0.5;
    } else if shape_type < 1.5 {
        let p = abs(uv - vec2<f32>(0.5, 0.5));
        return max(p.x, p.y) / 0.5;
    } else {
        let l0 = 1.0 - uv.x - 0.5 * uv.y;
        let l1 = uv.x - 0.5 * uv.y;
        let l2 = uv.y;
        return 1.0 - min(l0, min(l1, l2)) * 3.0;
    }
}

// ── Rotating 3-line star ────────────────────────────────────────
fn aim_star(uv: vec2<f32>, angle: f32, radius: f32, thickness: f32, inner_cut: f32) -> f32 {
    let p = uv - vec2<f32>(0.5, 0.5);
    let r = length(p);
    if r > radius || r < inner_cut { return 0.0; }
    let a = atan2(p.y, p.x) + angle;
    return 1.0 - smoothstep(0.0, thickness, abs(sin(a * 3.0)) * r);
}

// ── Distance → band intensity ───────────────────────────────────
fn shape_band(d: f32, shape_type: f32) -> f32 {
    if shape_type < 0.5 {
        return smoothstep(HEX_BAND_INNER, HEX_BAND_PEAK, d)
             * (1.0 - smoothstep(HEX_BAND_FADE, HEX_BAND_OUTER, d));
    }
    return smoothstep(GAP_BAND_START, 1.0, d);
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
    let d = shape_dist(in.uv, shape_type);
    let band = shape_band(d, shape_type);

    // Gap faces: darken edges slightly (keeps opaque batching, no alpha sorting)
    if shape_type > 0.5 {
        let edge_dim = 1.0 - smoothstep(EDGE_DIM_START, 1.0, d) * EDGE_DIM_STRENGTH;
        pbr_input.material.base_color = vec4<f32>(
            pbr_input.material.base_color.rgb * edge_dim,
            pbr_input.material.base_color.a
        );
    }

    // FoV overlay, scaled by progress
    if progress > 0.0 {
        if shape_type < 0.5 {
            let intensity = band * progress * HEX_FOV_TINT;
            pbr_input.material.base_color += vec4<f32>(intensity, intensity, intensity, 0.0);
            let em = band * progress * HEX_FOV_EMISSIVE;
            pbr_input.material.emissive += vec4<f32>(em, em, em, 0.0);
        } else {
            let intensity = band * progress * GAP_FOV_TINT;
            pbr_input.material.base_color += vec4<f32>(
                GAP_FOV_CYAN.x * intensity, GAP_FOV_CYAN.y * intensity, GAP_FOV_CYAN.z * intensity, 0.0
            );
        }
    }

    // Aim/Fire: orange underglow + rotating red-orange star
    if aim_mode > 0.5 {
        let firing = aim_mode > 1.5;
        let angle = globals.time * fov_overlay.data.w;
        let radius = aim_data.aim_params.x;
        let inner_cut = aim_data.aim_params.y;
        let thickness = aim_data.aim_params.z;

        // Orange gradient underneath
        let p = in.uv - vec2<f32>(0.5, 0.5);
        let r = length(p);
        let glow = smoothstep(radius + GLOW_MARGIN, 0.0, r);
        pbr_input.material.base_color += vec4<f32>(
            AIM_GLOW_RGB.x * glow, AIM_GLOW_RGB.y * glow, AIM_GLOW_RGB.z * glow, 0.0
        );

        // Red-orange star (redder when firing)
        let star = aim_star(in.uv, angle, radius, thickness, inner_cut);
        let star_color = select(AIM_STAR_COLOR, FIRE_STAR_COLOR, firing);
        pbr_input.material.base_color = mix(
            pbr_input.material.base_color,
            star_color,
            star
        );
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
