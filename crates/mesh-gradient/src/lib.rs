//! Procedural gradient materials for gap meshes between hex tiles.
//!
//! Generates [`StandardMaterial`]s with gradient textures that blend
//! between two (quad) or three (tri) input materials. The result is a
//! single inspectable material — no custom shaders or vertex colors.

use bevy::color::Mix;
use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Tri corner falloff curve for 3-way blending.
#[derive(Clone, Copy, Debug, Default)]
pub enum TriFalloff {
    /// `1 - hotspot(t)` — sharp plateaus at corners, narrow transition band,
    /// but creates dead-zone rings where all weights hit zero.
    Hotspot,
    /// Flat plateau at corners (weight=1.0 up to the hotspot edge), then
    /// quadratic decay toward the center. Sharp corner identity with a
    /// smooth tail that never creates dead zones.
    #[default]
    Plateau,
}

/// Blend gradient configuration.
#[derive(Resource)]
pub struct BlendCfg {
    /// Width of the transition band (0.0–1.0). Default: 0.15.
    pub band: f32,
    /// Quad texture dimensions `[width, height]`. Default: `[32, 16]`.
    pub quad_size: [u32; 2],
    /// Tri texture size (square). Default: 32.
    pub tri_size: u32,
}

impl Default for BlendCfg {
    fn default() -> Self {
        Self {
            band: 0.15,
            quad_size: [32, 16],
            tri_size: 64,
        }
    }
}

/// Blends two materials into a quad gradient material.
///
/// Produces a [`StandardMaterial`] with `base_color_texture` (sRGB gradient)
/// and `metallic_roughness_texture` (linear PBR gradient). The gradient runs
/// along the U axis: u=0 → material `a`, u=1 → material `b`.
pub fn blend_quad(
    a: &StandardMaterial,
    b: &StandardMaterial,
    cfg: &BlendCfg,
    images: &mut Assets<Image>,
) -> StandardMaterial {
    let [w, h] = cfg.quad_size;
    let a_lin = LinearRgba::from(a.base_color);
    let b_lin = LinearRgba::from(b.base_color);
    let band = cfg.band;

    let color_tex = mk_image(w, h, TextureFormat::Rgba8UnormSrgb, |u, _| {
        let t = hotspot(u, band);
        srgb_bytes(a_lin.mix(&b_lin, t))
    });

    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(images.add(color_tex)),
        metallic: (a.metallic + b.metallic) / 2.0,
        perceptual_roughness: (a.perceptual_roughness + b.perceptual_roughness) / 2.0,
        cull_mode: None,
        ..default()
    }
}

/// Blends three materials into a tri gradient material.
///
/// `base_idx` (0, 1, or 2) selects the center fallback corner.
/// `falloff` controls corner sharpness (see [`TriFalloff`]).
pub fn blend_tri(
    mats: [&StandardMaterial; 3],
    base_idx: usize,
    falloff: TriFalloff,
    cfg: &BlendCfg,
    images: &mut Assets<Image>,
) -> StandardMaterial {
    let sz = cfg.tri_size;
    let colors: [LinearRgba; 3] = mats.map(|m| LinearRgba::from(m.base_color));
    let roughness: [f32; 3] = mats.map(|m| m.perceptual_roughness);
    let metallic: [f32; 3] = mats.map(|m| m.metallic);
    let band = cfg.band;

    let color_tex = mk_image(sz, sz, TextureFormat::Rgba8UnormSrgb, |u, v| {
        srgb_bytes(tri_blend(&colors, [u, v], band, base_idx, falloff))
    });

    let avg_r = roughness.iter().sum::<f32>() / 3.0;
    let avg_m = metallic.iter().sum::<f32>() / 3.0;

    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(images.add(color_tex)),
        metallic: avg_m,
        perceptual_roughness: avg_r,
        cull_mode: None,
        ..default()
    }
}

// ── Internal helpers ───────────────────────────────────────────────

// sRGB transfer function constants (IEC 61966-2-1).
const SRGB_LINEAR_CUTOFF: f32 = 0.0031308;
const SRGB_LINEAR_SCALE: f32 = 12.92;
const SRGB_GAMMA: f32 = 2.4;
const SRGB_A: f32 = 0.055;

fn hotspot(t: f32, band: f32) -> f32 {
    if band <= 0.0 {
        return if t < 0.5 { 0.0 } else { 1.0 };
    }
    let lo = 0.5 - band / 2.0;
    let hi = 0.5 + band / 2.0;
    ((t - lo) / (hi - lo)).clamp(0.0, 1.0)
}

/// Linear RGB → sRGB-encoded RGBA bytes.
fn srgb_bytes(c: LinearRgba) -> [u8; 4] {
    let to_srgb = |v: f32| -> u8 {
        let s = if v <= SRGB_LINEAR_CUTOFF {
            v * SRGB_LINEAR_SCALE
        } else {
            (1.0 + SRGB_A) * v.powf(1.0 / SRGB_GAMMA) - SRGB_A
        };
        (s.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
    };
    [to_srgb(c.red), to_srgb(c.green), to_srgb(c.blue), 255]
}

/// Generates a texture from a per-pixel function over normalized UV coords.
fn mk_image(w: u32, h: u32, fmt: TextureFormat, pixel: impl Fn(f32, f32) -> [u8; 4]) -> Image {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let v = if h > 1 {
            y as f32 / (h - 1) as f32
        } else {
            0.5
        };
        for x in 0..w {
            let u = if w > 1 {
                x as f32 / (w - 1) as f32
            } else {
                0.5
            };
            data.extend_from_slice(&pixel(u, v));
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        fmt,
        default(),
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        min_filter: ImageFilterMode::Linear,
        mag_filter: ImageFilterMode::Linear,
        ..default()
    });
    img
}

// ── Tri UV geometry ────────────────────────────────────────────────

/// UV positions matching gap mesh vertex order: v0=(0,0), v1=(1,0), v2=(0.5,1).
const TRI_UVS: [[f32; 2]; 3] = [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];

/// Precomputed midpoint of the edge opposite each corner in UV space.
/// `TRI_OPP_MIDS[i]` = midpoint of edge between corners `(i+1)%3` and `(i+2)%3`.
const TRI_OPP_MIDS: [[f32; 2]; 3] = [[0.75, 0.5], [0.25, 0.5], [0.5, 0.0]];

/// Projects a UV point onto the gradient axis from `corner` toward its
/// opposite edge midpoint. Returns t: 0 = at corner, 1 = at midpoint.
fn tri_project(uv: [f32; 2], corner: usize) -> f32 {
    let c = TRI_UVS[corner];
    let m = TRI_OPP_MIDS[corner];
    let dx = m[0] - c[0];
    let dy = m[1] - c[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-8 {
        return 0.0;
    }
    ((uv[0] - c[0]) * dx + (uv[1] - c[1]) * dy) / len_sq
}

/// Normalized 3-way blend dispatching on [`TriFalloff`].
fn tri_blend(
    colors: &[LinearRgba; 3],
    uv: [f32; 2],
    band: f32,
    base_idx: usize,
    falloff: TriFalloff,
) -> LinearRgba {
    let w: [f32; 3] = std::array::from_fn(|i| {
        let t = tri_project(uv, i).clamp(0.0, 1.0);
        match falloff {
            TriFalloff::Hotspot => 1.0 - hotspot(t, band),
            TriFalloff::Plateau => {
                let lo = (0.5 - band / 2.0).max(0.0);
                if t <= lo {
                    1.0
                } else {
                    let tail = (1.0 - t) / (1.0 - lo);
                    tail * tail
                }
            }
        }
    });
    let total = w[0] + w[1] + w[2];
    if total < 1e-6 {
        return colors[base_idx];
    }
    LinearRgba::new(
        (colors[0].red * w[0] + colors[1].red * w[1] + colors[2].red * w[2]) / total,
        (colors[0].green * w[0] + colors[1].green * w[1] + colors[2].green * w[2]) / total,
        (colors[0].blue * w[0] + colors[1].blue * w[1] + colors[2].blue * w[2]) / total,
        1.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotspot_boundaries() {
        let b = 0.15;
        assert_eq!(hotspot(0.0, b), 0.0);
        assert_eq!(hotspot(0.42, b), 0.0);
        assert!((hotspot(0.5, b) - 0.5).abs() < 1e-4);
        assert_eq!(hotspot(0.58, b), 1.0);
        assert_eq!(hotspot(1.0, b), 1.0);
    }

    #[test]
    fn hotspot_zero_band_is_step() {
        assert_eq!(hotspot(0.49, 0.0), 0.0);
        assert_eq!(hotspot(0.5, 0.0), 1.0);
        assert_eq!(hotspot(0.51, 0.0), 1.0);
    }

    #[test]
    fn tri_opp_mids_correct() {
        // Verify precomputed midpoints match expected values.
        assert!((TRI_OPP_MIDS[0][0] - 0.75).abs() < 1e-6);
        assert!((TRI_OPP_MIDS[0][1] - 0.5).abs() < 1e-6);
        assert!((TRI_OPP_MIDS[2][0] - 0.5).abs() < 1e-6);
        assert!((TRI_OPP_MIDS[2][1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn tri_project_at_corner_is_zero() {
        for i in 0..3 {
            let t = tri_project(TRI_UVS[i], i);
            assert!(t.abs() < 1e-6, "corner {i}: expected ~0, got {t}");
        }
    }

    #[test]
    fn tri_project_at_opp_mid_is_one() {
        for i in 0..3 {
            let t = tri_project(TRI_OPP_MIDS[i], i);
            assert!((t - 1.0).abs() < 1e-4, "corner {i}: expected ~1, got {t}");
        }
    }

    #[test]
    fn srgb_bytes_black_and_white() {
        assert_eq!(srgb_bytes(LinearRgba::BLACK)[..3], [0, 0, 0]);
        assert_eq!(srgb_bytes(LinearRgba::WHITE)[..3], [255, 255, 255]);
    }

    #[test]
    fn blend_quad_same_materials() {
        let mut images = Assets::<Image>::default();
        let mat = StandardMaterial {
            base_color: Color::srgb(0.5, 0.3, 0.1),
            perceptual_roughness: 0.8,
            metallic: 0.2,
            ..default()
        };
        let result = blend_quad(&mat, &mat, &BlendCfg::default(), &mut images);
        assert!(result.base_color_texture.is_some());
        assert_eq!(result.cull_mode, None);
        assert!((result.metallic - 0.2).abs() < 1e-6);
        assert!((result.perceptual_roughness - 0.8).abs() < 1e-6);
    }

    #[test]
    fn blend_tri_same_materials() {
        let mut images = Assets::<Image>::default();
        let mat = StandardMaterial {
            base_color: Color::srgb(0.5, 0.3, 0.1),
            perceptual_roughness: 0.8,
            metallic: 0.2,
            ..default()
        };
        let result = blend_tri(
            [&mat, &mat, &mat],
            0,
            TriFalloff::default(),
            &BlendCfg::default(),
            &mut images,
        );
        assert!(result.base_color_texture.is_some());
        assert!((result.metallic - 0.2).abs() < 1e-6);
    }

    #[test]
    fn mk_image_dimensions() {
        let img = mk_image(16, 8, TextureFormat::Rgba8Unorm, |_, _| {
            [128, 128, 128, 255]
        });
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 8);
        assert_eq!(img.data.as_ref().unwrap().len(), 16 * 8 * 4);
    }
}
