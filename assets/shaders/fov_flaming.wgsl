// Cyan flame overlay for InFov gap faces (Quad/Tri).
// FBM noise flames lick inward from all edges, center shows base material.

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

// ── Uniforms ───────────────────────────────────────────────────
struct FovOverlayData { data: vec4<f32>, }
struct AimParamsData  { aim_params: vec4<f32>, } // layout compat with FovOverlay

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> fov_overlay: FovOverlayData;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> aim_data: AimParamsData;

// ── Constants ──────────────────────────────────────────────────
const EDGE_THRESHOLD: f32 = 0.7;
const EDGE_DIM_STRENGTH: f32 = 1.0 / 9.0;

const FLAME_REACH: f32 = 0.3;
const FLAME_SPEED: f32 = 1.5;
const FLAME_SCALE: f32 = 8.0;
const FLAME_OCTAVES: i32 = 4;
const FLAME_SHARPNESS: f32 = 2.5;
const FLAME_CYAN: vec3<f32> = vec3<f32>(0.3, 1.0, 1.0);
const FLAME_TINT: f32 = 0.5;
const FLAME_EMISSIVE: f32 = 0.25;

// ── Sin-free hashing ───────────────────────────────────────────
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// ── Value noise ────────────────────────────────────────────────
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f); // smoothstep interpolation

    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// ── FBM (4 octaves) ───────────────────────────────────────────
fn fbm(p: vec2<f32>) -> f32 {
    var val: f32 = 0.0;
    var amp: f32 = 0.5;
    var pos = p;
    for (var i: i32 = 0; i < FLAME_OCTAVES; i++) {
        val += amp * noise(pos);
        pos *= 2.0;
        amp *= 0.5;
    }
    return val;
}

// ── Shape distance: 0 at center, 1 at edge ─────────────────────
fn shape_dist(uv: vec2<f32>, shape_type: f32) -> f32 {
    if shape_type < 1.5 {
        let p = abs(uv - vec2<f32>(0.5, 0.5));
        return max(p.x, p.y) / 0.5;
    } else {
        let l0 = 1.0 - uv.x - 0.5 * uv.y;
        let l1 = uv.x - 0.5 * uv.y;
        let l2 = uv.y;
        return 1.0 - min(l0, min(l1, l2)) * 3.0;
    }
}

// ── Flame intensity from edge ──────────────────────────────────
fn flame(uv: vec2<f32>, d: f32, time: f32) -> f32 {
    let edge_band = smoothstep(1.0 - FLAME_REACH, 1.0, d);

    // Swirl warp: rotates noise coords over time for non-directional flicker
    let t = time * FLAME_SPEED;
    let c = cos(t * 0.3);
    let s = sin(t * 0.3);
    let swirl = vec2<f32>(uv.x * c - uv.y * s, uv.x * s + uv.y * c);

    // Two noise layers at different scales for richer detail
    let n1 = fbm(swirl * FLAME_SCALE + vec2<f32>(0.0, t));
    let n2 = fbm(swirl * FLAME_SCALE * 0.5 - vec2<f32>(t * 0.7, 0.0));
    let n = (n1 + n2) * 0.5;

    // Sharpen peaks for flame-like tips
    let sharp = pow(max(n, 0.0), FLAME_SHARPNESS);

    return edge_band * sharp;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let progress = fov_overlay.data.x;
    let shape_type = fov_overlay.data.z;
    let d = shape_dist(in.uv, shape_type);

    let edge_dim = 1.0 - smoothstep(EDGE_THRESHOLD, 1.0, d) * EDGE_DIM_STRENGTH;
    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * edge_dim,
        pbr_input.material.base_color.a
    );

    if progress > 0.0 {
        let f = flame(in.uv, d, globals.time) * progress;
        pbr_input.material.base_color += vec4<f32>(FLAME_CYAN * f * FLAME_TINT, 0.0);
        pbr_input.material.emissive += vec4<f32>(FLAME_CYAN * f * FLAME_EMISSIVE, 0.0);
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
