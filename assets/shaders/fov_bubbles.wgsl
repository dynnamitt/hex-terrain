// Bubble dissolve overlay for InFov gap faces (Quad/Tri).
// Bubbles spawn at edges, drift inward, shrink + fade.
// Center shows untouched base material.

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

// ── Uniforms (same layout as aiming_overlay) ───────────────────
// data (binding 100): x=fov_progress, y=unused, z=shape_type (1=quad, 2=tri), w=unused
struct FovOverlayData { data: vec4<f32>, }
struct AimParamsData  { aim_params: vec4<f32>, }

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> fov_overlay: FovOverlayData;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> aim_data: AimParamsData;

// ── Gap edge darkening ─────────────────────────────────────────
const EDGE_DIM_START: f32 = 0.7;
const EDGE_DIM_STRENGTH: f32 = 1.0 / 9.0;

// ── Bubble tuning ──────────────────────────────────────────────
const GRID_SCALE: f32 = 5.0;
const DRIFT_SPEED: f32 = 0.6;
const BUBBLE_MIN_R: f32 = 0.06;
const BUBBLE_MAX_R: f32 = 0.22;
const SHRINK_FACTOR: f32 = 0.7;
const AA_WIDTH: f32 = 0.003;
const BUBBLE_TINT: f32 = 0.12;
const BUBBLE_EMISSIVE: f32 = 0.03;

// ── Hashing ────────────────────────────────────────────────────
fn hash21(p: vec2<f32>) -> f32 {
    var s = dot(p, vec2<f32>(127.1, 311.7));
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
        // Quad: box distance
        let p = abs(uv - vec2<f32>(0.5, 0.5));
        return max(p.x, p.y) / 0.5;
    } else {
        // Tri: barycentric distance
        let l0 = 1.0 - uv.x - 0.5 * uv.y;
        let l1 = uv.x - 0.5 * uv.y;
        let l2 = uv.y;
        return 1.0 - min(l0, min(l1, l2)) * 3.0;
    }
}

// ── Bubble field ───────────────────────────────────────────────
fn bubble_field(uv: vec2<f32>, shape_type: f32, time: f32, progress: f32) -> f32 {
    let d = shape_dist(uv, shape_type);
    let grid_uv = uv * GRID_SCALE;
    let cell = floor(grid_uv);
    var result: f32 = 0.0;

    // 3x3 neighbor scan
    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            let neighbor = cell + vec2<f32>(f32(dx), f32(dy));
            let h_pos = hash22(neighbor);
            let h_val = hash21(neighbor + vec2<f32>(42.0, 17.0));

            // Bubble center within cell
            let bubble_center = (neighbor + 0.3 + h_pos * 0.4) / GRID_SCALE;

            // Birth distance from edge (0.4..0.95)
            let birth_d = 0.4 + h_val * 0.55;

            // Phase offset per bubble for staggered drift
            let phase = hash21(neighbor + vec2<f32>(7.0, 13.0));

            // Lifecycle: wraps with fract for seamless loop
            let life = fract(time * DRIFT_SPEED + phase);

            // Current distance from edge: drifts from birth_d toward 0
            let current_d = birth_d * (1.0 - life);

            // Only draw if bubble is within the solid zone (near edges)
            if current_d > d { continue; }

            // Life progress: 0 = just born at edge, 1 = at center
            let life_t = life;

            // Radius: shrinks as bubble ages
            let base_r = (BUBBLE_MIN_R + h_val * (BUBBLE_MAX_R - BUBBLE_MIN_R)) / GRID_SCALE;
            let radius = base_r * (1.0 - life_t * SHRINK_FACTOR);

            // Fade as bubble approaches center
            let alpha = 1.0 - life_t;

            // Distance from fragment to bubble center
            let dist = length(uv - bubble_center);

            // Sharp circle with slight AA
            let mask = 1.0 - smoothstep(radius - AA_WIDTH, radius, dist);

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

    // Gap edge darkening (keeps opaque batching, no alpha sorting)
    let edge_dim = 1.0 - smoothstep(EDGE_DIM_START, 1.0, d) * EDGE_DIM_STRENGTH;
    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * edge_dim,
        pbr_input.material.base_color.a
    );

    // Bubble dissolve overlay
    if progress > 0.0 {
        let bubble = bubble_field(in.uv, shape_type, globals.time, progress);
        let tint = bubble * BUBBLE_TINT;
        pbr_input.material.base_color += vec4<f32>(tint, tint, tint, 0.0);
        let em = bubble * BUBBLE_EMISSIVE;
        pbr_input.material.emissive += vec4<f32>(em, em, em, 0.0);
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
