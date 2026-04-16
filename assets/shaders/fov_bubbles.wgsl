// Bubble dissolve overlay for InFov gap faces (Quad/Tri).
// Bubbles spawn at edge band, drift inward, shrink + fade.

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
struct AimParamsData  { aim_params: vec4<f32>, } // unused — layout compat with FovOverlay

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> fov_overlay: FovOverlayData;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> aim_data: AimParamsData;

// ── Constants ──────────────────────────────────────────────────
const EDGE_THRESHOLD: f32 = 0.7;
const EDGE_DIM_STRENGTH: f32 = 1.0 / 9.0;

const GRID_SCALE: f32 = 4.0;
const DRIFT_SPEED: f32 = 0.25;
const BUBBLE_START_R: f32 = 0.18;
const AA_WIDTH: f32 = 0.004;
const TINT_STRENGTH: f32 = 0.18;
const EMISSIVE_STRENGTH: f32 = 0.05;
const CYAN: vec3<f32> = vec3<f32>(0.3, 1.0, 1.0);

// Stepped band: 4 opacity steps from edge inward
const BAND_STEPS: array<f32, 4> = array<f32, 4>(1.0, 0.76, 0.41, 0.13);
const BAND_WIDTH: f32 = 0.3;
const SPAWN_D: f32 = 0.925; // EDGE_THRESHOLD + 0.75 * BAND_WIDTH

// ── Hashing ────────────────────────────────────────────────────
fn hash21(p: vec2<f32>) -> f32 {
    let s = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(s) * 43758.5453);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let x = dot(p, vec2<f32>(127.1, 311.7));
    let y = dot(p, vec2<f32>(269.5, 183.3));
    return fract(sin(vec2<f32>(x, y)) * 43758.5453);
}

// ── Shape distance: 0 at center, 1 at edge ─────────────────────
fn shape_dist(uv: vec2<f32>, shape_type: f32) -> f32 {
    if shape_type < 1.5 {
        // Quad
        let p = abs(uv - vec2<f32>(0.5, 0.5));
        return max(p.x, p.y) / 0.5;
    } else {
        // Tri (barycentric)
        let l0 = 1.0 - uv.x - 0.5 * uv.y;
        let l1 = uv.x - 0.5 * uv.y;
        let l2 = uv.y;
        return 1.0 - min(l0, min(l1, l2)) * 3.0;
    }
}

// ── Stepped band opacity ───────────────────────────────────────
fn band_opacity(d: f32) -> f32 {
    if d < EDGE_THRESHOLD { return 0.0; }
    let t = (d - EDGE_THRESHOLD) / BAND_WIDTH;
    let idx = clamp(i32(t * 4.0), 0, 3);
    return BAND_STEPS[3 - idx]; // outermost = highest
}

// ── Bubble field ───────────────────────────────────────────────
fn bubble_field(uv: vec2<f32>, shape_type: f32, time: f32, progress: f32) -> f32 {
    let grid_uv = uv * GRID_SCALE;
    let cell = floor(grid_uv);
    let center = vec2<f32>(0.5, 0.5);
    let start_r = BUBBLE_START_R / GRID_SCALE;
    var result: f32 = 0.0;

    for (var dy: i32 = -2; dy <= 2; dy++) {
        for (var dx: i32 = -2; dx <= 2; dx++) {
            let neighbor = cell + vec2<f32>(f32(dx), f32(dy));
            let h_pos = hash22(neighbor);

            let raw_origin = (neighbor + 0.2 + h_pos * 0.6) / GRID_SCALE;
            let d_raw = shape_dist(raw_origin, shape_type);
            let origin = center + (raw_origin - center) * SPAWN_D / max(d_raw, 0.001);

            let phase = hash21(neighbor + vec2<f32>(7.0, 13.0));
            let life = fract(time * DRIFT_SPEED + phase);
            let pos = mix(origin, center, life);
            let radius = start_r * (1.0 - life);
            let alpha = 1.0 - life;

            let mask = 1.0 - smoothstep(radius - AA_WIDTH, radius, length(uv - pos));
            result = max(result, mask * alpha);
        }
    }

    return result * progress;
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

    // Edge darkening (opaque, no alpha sorting)
    let edge_dim = 1.0 - smoothstep(EDGE_THRESHOLD, 1.0, d) * EDGE_DIM_STRENGTH;
    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * edge_dim,
        pbr_input.material.base_color.a
    );

    if progress > 0.0 {
        let band = band_opacity(d) * progress * TINT_STRENGTH;
        let bubble = bubble_field(in.uv, shape_type, globals.time, progress);
        let tint = max(band, bubble * TINT_STRENGTH);
        pbr_input.material.base_color += vec4<f32>(CYAN * tint, 0.0);
        pbr_input.material.emissive += vec4<f32>(CYAN * bubble * EMISSIVE_STRENGTH, 0.0);
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
