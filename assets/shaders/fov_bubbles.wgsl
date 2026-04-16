// Animated Voronoi overlay for InFov gap faces (Quad/Tri).
// Cyan cell borders, 3-tier base-material brightness in interiors.

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

const VORONOI_COLS: f32 = 5.0;
const QUAD_ROWS: f32 = 2.0;
const TRI_ROWS: f32 = 5.0;
const ANIM_SPEED: f32 = 0.3;
const BORDER_WIDTH: f32 = 0.1;
const BORDER_CYAN: vec3<f32> = vec3<f32>(0.3, 1.0, 1.0);
const BORDER_TINT: f32 = 0.01;
const BORDER_EMISSIVE: f32 = 0.18;
const CELL_ALPHAS: array<f32, 3> = array<f32, 3>(0.0, 0.04, 0.08);

// ── Sin-free hashing (portable across GPUs) ────────────────────
fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
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

// ── Animated Voronoi ───────────────────────────────────────────
struct VoronoiResult {
    border: f32,
    cell_alpha: f32,
}

fn voronoi(uv: vec2<f32>, time: f32, shape_type: f32) -> VoronoiResult {
    let rows = select(TRI_ROWS, QUAD_ROWS, shape_type < 1.5);
    let grid_uv = uv * vec2<f32>(VORONOI_COLS, rows);
    let cell = floor(grid_uv);
    let frac = fract(grid_uv);

    var d1: f32 = 8.0; // nearest
    var d2: f32 = 8.0; // second nearest
    var nearest_cell: vec2<f32>;

    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            let neighbor = vec2<f32>(f32(dx), f32(dy));
            let n_cell = cell + neighbor;
            let h = hash22(n_cell);

            // Animated center: oscillates around cell center
            let center = neighbor + 0.5 + 0.4 * sin(time * ANIM_SPEED + h * 6.2831) - frac;
            let dist = dot(center, center);

            if dist < d1 {
                d2 = d1;
                d1 = dist;
                nearest_cell = n_cell;
            } else if dist < d2 {
                d2 = dist;
            }
        }
    }

    let border = 1.0 - smoothstep(0.0, BORDER_WIDTH, sqrt(d2) - sqrt(d1));
    let tier = i32(hash21(nearest_cell) * 3.0);
    let cell_alpha = CELL_ALPHAS[clamp(tier, 0, 2)];

    return VoronoiResult(border, cell_alpha);
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
        let v = voronoi(in.uv, globals.time, shape_type);

        // Cell borders → cyan tint + emissive glow
        let border_tint = v.border * progress;
        pbr_input.material.base_color += vec4<f32>(BORDER_CYAN * border_tint * BORDER_TINT, 0.0);
        pbr_input.material.emissive += vec4<f32>(BORDER_CYAN * border_tint * BORDER_EMISSIVE, 0.0);

        // Cell interiors → subtle brightness variation
        pbr_input.material.base_color += vec4<f32>(vec3<f32>(v.cell_alpha * progress), 0.0);
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
