//! Procedural hex-mushroom mesh generation.
//!
//! Ports the Blender Python geometry (6-sided rings, sinusoidal bend,
//! variable radius, twist) to Bevy [`Mesh`] with Y-up orientation.

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::Mesh;
use bevy::render::render_resource::PrimitiveTopology;

use crate::FloraCfg;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Hex ring: 6 vertices at `(cx + r*cos, y, cz + r*sin)`.
fn hex_ring(cx: f32, y: f32, cz: f32, radius: f32, rot: f32, sides: u32, out: &mut Vec<[f32; 3]>) {
    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * TAU + rot;
        let (sin_a, cos_a) = angle.sin_cos();
        out.push([cx + cos_a * radius, y, cz + sin_a * radius]);
    }
}

// ── Stem ───────────────────────────────────────────────────────────

/// Variable radius along stem: base -> narrow (t=0..0.4) -> top (t=0.4..1).
fn stem_r(cfg: &FloraCfg, t: f32) -> f32 {
    let narrow_t = 0.4;
    if t < narrow_t {
        lerp(cfg.stem_base_r, cfg.stem_narrow_r, t / narrow_t)
    } else {
        lerp(
            cfg.stem_narrow_r,
            cfg.stem_top_r,
            (t - narrow_t) / (1.0 - narrow_t),
        )
    }
}

/// Sinusoidal XZ bend, anchored at base (multiplied by t).
fn stem_bend(cfg: &FloraCfg, t: f32) -> (f32, f32) {
    let x = cfg.stem_bend_x * (t * PI * 0.9).sin() * t;
    let z = cfg.stem_bend_z * (t * PI * 0.7 + 0.3).sin() * t;
    (x, z)
}

/// Build a hexagonal stem mesh.
///
/// - `stem_seg + 1` rings of `sides` vertices each
/// - Quad strips between rings + bottom cap fan
/// - Normals point radially outward from axis
pub fn build_stem(cfg: &FloraCfg) -> Mesh {
    let sides = cfg.sides;
    let n_rings = cfg.stem_seg + 1;
    let twist_total = cfg.stem_twist_deg.to_radians();

    let ring_verts = (n_rings * sides) as usize;
    let mut pos = Vec::with_capacity(ring_verts + 1);
    let mut nor = Vec::with_capacity(ring_verts + 1);
    let mut uvs = Vec::with_capacity(ring_verts + 1);
    let mut idx: Vec<u32> = Vec::new();

    // Generate ring vertices
    for ring in 0..n_rings {
        let t = ring as f32 / cfg.stem_seg as f32;
        let y = t * cfg.stem_height;
        let r = stem_r(cfg, t);
        let (bx, bz) = stem_bend(cfg, t);
        let twist = t * twist_total;

        let base = pos.len();
        hex_ring(bx, y, bz, r, twist, sides, &mut pos);

        // Radial outward normals (ignoring bend — subtle enough)
        for i in 0..sides {
            let angle = (i as f32 / sides as f32) * TAU + twist;
            let (sin_a, cos_a) = angle.sin_cos();
            nor.push([cos_a, 0.0, sin_a]);
            let u = (base as u32 + i) as f32 / sides as f32;
            uvs.push([u % 1.0, t]);
        }
    }

    // Quad indices between adjacent rings
    for seg in 0..cfg.stem_seg {
        let lo = seg * sides;
        let hi = lo + sides;
        for i in 0..sides {
            let i_next = (i + 1) % sides;
            idx.extend_from_slice(&[
                lo + i,
                hi + i,
                hi + i_next,
                lo + i,
                hi + i_next,
                lo + i_next,
            ]);
        }
    }

    // Bottom cap fan from center vertex
    let center = pos.len() as u32;
    pos.push([0.0, 0.0, 0.0]);
    nor.push([0.0, -1.0, 0.0]);
    uvs.push([0.5, 0.0]);

    for i in 0..sides {
        let i_next = (i + 1) % sides;
        idx.extend_from_slice(&[center, i_next, i]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(idx))
}

// ── Cap ────────────────────────────────────────────────────────────

/// Build a hexagonal cap mesh (dome + underside annulus).
///
/// Local origin is at the cap base (sits at stem top when parented).
/// - `cap_rings + 1` concentric hex rings converging to apex
/// - Triangle fan from innermost ring to apex vertex
/// - Underside annulus from outer ring back to stem-top width
pub fn build_cap(cfg: &FloraCfg) -> Mesh {
    let sides = cfg.sides;
    let n_dome = cfg.cap_rings + 1; // ring count including outer
    let twist = cfg.stem_twist_deg.to_radians(); // match stem top rotation

    // Estimate: dome rings + apex + underside ring
    let est = ((n_dome + 1) * sides + 1) as usize;
    let mut pos = Vec::with_capacity(est);
    let mut nor = Vec::with_capacity(est);
    let mut uvs = Vec::with_capacity(est);
    let mut idx: Vec<u32> = Vec::new();

    // Bend offset at stem top (cap base aligns with stem top)
    let (bx, bz) = stem_bend(cfg, 1.0);

    // Dome rings
    for ring in 0..n_dome {
        let t = ring as f32 / cfg.cap_rings as f32;
        let r = cfg.cap_r * (1.0 - t.powf(1.3));
        let y = cfg.cap_h * (1.0 - (1.0 - t).powf(2.5));

        hex_ring(bx, y, bz, r.max(0.02), twist, sides, &mut pos);

        // Normals: blend outward → upward
        for i in 0..sides {
            let angle = (i as f32 / sides as f32) * TAU + twist;
            let (sin_a, cos_a) = angle.sin_cos();
            let up = t;
            let nx = cos_a * (1.0 - up);
            let ny = up;
            let nz = sin_a * (1.0 - up);
            let len = (nx * nx + ny * ny + nz * nz).sqrt().max(f32::EPSILON);
            nor.push([nx / len, ny / len, nz / len]);
            uvs.push([i as f32 / sides as f32, t]);
        }
    }

    // Apex vertex
    let apex = pos.len() as u32;
    pos.push([bx, cfg.cap_h, bz]);
    nor.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 1.0]);

    // Quad strips between dome rings
    for ring in 0..cfg.cap_rings {
        let outer = ring * sides;
        let inner = outer + sides;
        for i in 0..sides {
            let i_next = (i + 1) % sides;
            idx.extend_from_slice(&[
                outer + i,
                inner + i,
                inner + i_next,
                outer + i,
                inner + i_next,
                outer + i_next,
            ]);
        }
    }

    // Fan from last ring to apex
    let last = cfg.cap_rings * sides;
    for i in 0..sides {
        let i_next = (i + 1) % sides;
        idx.extend_from_slice(&[last + i, apex, last + i_next]);
    }

    // Underside annulus: stem-width ring → outer cap ring
    let under_base = pos.len() as u32;
    hex_ring(bx, -0.01, bz, cfg.stem_top_r, twist, sides, &mut pos);
    for _ in 0..sides {
        nor.push([0.0, -1.0, 0.0]);
        uvs.push([0.0, 0.0]);
    }

    for i in 0..sides {
        let i_next = (i + 1) % sides;
        // Outer ring index (ring 0)
        let o = i;
        let on = i_next;
        let u = under_base + i;
        let un = under_base + i_next;
        // Winding for downward-facing
        idx.extend_from_slice(&[o, u, un, o, un, on]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cfg() -> FloraCfg {
        FloraCfg::default()
    }

    #[test]
    fn stem_vert_count() {
        let m = build_stem(&default_cfg());
        let pos = m
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        // 11 rings * 6 verts + 1 center = 67
        assert_eq!(pos.len(), 67);
    }

    #[test]
    fn cap_vert_count() {
        let m = build_cap(&default_cfg());
        let pos = m
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        // 4 rings * 6 + 1 apex + 6 underside = 31
        assert_eq!(pos.len(), 31);
    }

    #[test]
    fn stem_has_all_attrs() {
        let m = build_stem(&default_cfg());
        assert!(m.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(m.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(m.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
        assert!(m.indices().is_some());
    }

    #[test]
    fn cap_has_all_attrs() {
        let m = build_cap(&default_cfg());
        assert!(m.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(m.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(m.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
        assert!(m.indices().is_some());
    }
}
