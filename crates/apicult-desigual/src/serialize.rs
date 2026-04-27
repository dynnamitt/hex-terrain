//! Serialization implementations for [`HGridLayout`] geometry.
//!
//! Each format implements [`SerializeGeo`] and can stream output to any
//! `io::Write`. The example binary is a thin CLI wrapper that selects an
//! implementor by `--format` flag.

use std::io;

use glam::{Vec2, Vec3};
use hexx::{Hex, shapes};

use crate::HGridLayout;

/// Streamed serialization of an apicult-desigual layout to bytes.
pub trait SerializeGeo {
    fn write(&self, layout: &HGridLayout, out: &mut dyn io::Write) -> io::Result<()>;
}

/// SVG with monochrome hex fills and a single stroke pass.
pub struct SvgPlain {
    pub pad: f32,
}

/// SVG with green-tinted hex fills, dual-stroke outline, and per-hex height labels.
pub struct SvgRich {
    pub pad: f32,
}

/// JSON v1 (gen1): `{ hexes, edges, quads, tris }`. Backward-compatible payload.
pub struct JsonV1;

/// JSON v2 (gen2): `{ version: 2, tris }`. Unified triangle stream from
/// [`HGridLayout::all_tris`] — diagonal choice owned in rust, no per-source toggle.
pub struct JsonV2;

// ─── Internal helpers ─────────────────────────────────────────────────

struct HexData {
    center: Vec2,
    corners: [(f32, f32); 6],
    height: f32,
}

struct SvgFrame {
    hex_data: Vec<HexData>,
    long_edges: Vec<(Vec3, Vec3)>,
    max_h: f32,
    vb_x: f32,
    vb_z: f32,
    vb_w: f32,
    vb_h: f32,
}

fn collect_frame(layout: &HGridLayout, pad: f32) -> SvgFrame {
    let hexes: Vec<Hex> = shapes::hexagon(Hex::ZERO, layout.grid_radius()).collect();
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut max_h: f32 = 0.0;

    let hex_data: Vec<HexData> = hexes
        .iter()
        .filter_map(|&hex| {
            let center = layout.hex_to_world_pos(hex);
            let height = layout.height(&hex)?;
            max_h = max_h.max(height);
            let mut corners = [(0.0f32, 0.0f32); 6];
            for i in 0..6u8 {
                let v = layout.vertex(hex, i)?;
                corners[i as usize] = (v.x, v.z);
                min_x = min_x.min(v.x);
                max_x = max_x.max(v.x);
                min_z = min_z.min(v.z);
                max_z = max_z.max(v.z);
            }
            Some(HexData {
                center,
                corners,
                height,
            })
        })
        .collect();

    SvgFrame {
        hex_data,
        long_edges: layout.quad_long_edges(),
        max_h,
        vb_x: min_x - pad,
        vb_z: min_z - pad,
        vb_w: (max_x - min_x) + 2.0 * pad,
        vb_h: (max_z - min_z) + 2.0 * pad,
    }
}

fn points_of(corners: &[(f32, f32); 6]) -> String {
    corners
        .iter()
        .map(|(x, z)| format!("{x:.2},{z:.2}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_svg(
    layout: &HGridLayout,
    out: &mut dyn io::Write,
    pad: f32,
    rich: bool,
) -> io::Result<()> {
    let f = collect_frame(layout, pad);
    let stroke = 0.12;
    let outline = stroke * 3.0;

    writeln!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{:.1} {:.1} {:.1} {:.1}" width="800" height="800">"##,
        f.vb_x, f.vb_z, f.vb_w, f.vb_h,
    )?;

    for hd in &f.hex_data {
        let t = if f.max_h > 0.0 {
            hd.height / f.max_h
        } else {
            0.0
        };
        let (r, g, b) = if rich {
            (
                (40.0 + t * 80.0) as u8,
                (60.0 + t * 180.0) as u8,
                (30.0 + t * 40.0) as u8,
            )
        } else {
            let v = (60.0 + t * 160.0) as u8;
            (v, v, v)
        };
        let points = points_of(&hd.corners);
        writeln!(
            out,
            r##"  <polygon points="{points}" fill="rgb({r},{g},{b})" stroke="none" opacity="0.9"/>"##,
        )?;
    }

    let hex_strokes: &[(&str, f32)] = if rich {
        &[("white", outline), ("black", stroke)]
    } else {
        &[("gray", stroke)]
    };
    for (color, width) in hex_strokes {
        for hd in &f.hex_data {
            let points = points_of(&hd.corners);
            writeln!(
                out,
                r##"  <polygon points="{points}" fill="none" stroke="{color}" stroke-width="{width:.2}" stroke-linejoin="round"/>"##,
            )?;
        }
    }

    for (color, width) in hex_strokes {
        for (from, to) in &f.long_edges {
            writeln!(
                out,
                r##"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{color}" stroke-width="{width:.2}" stroke-linecap="round"/>"##,
                from.x, from.z, to.x, to.z,
            )?;
        }
    }

    if rich {
        let font = layout.point_spacing() * 0.22;
        for hd in &f.hex_data {
            writeln!(
                out,
                r##"  <text x="{:.2}" y="{:.2}" font-size="{font:.2}" font-family="monospace" fill="black" text-anchor="middle" dominant-baseline="central">{:.1}</text>"##,
                hd.center.x, hd.center.y, hd.height,
            )?;
        }
    }

    writeln!(out, "</svg>")?;
    Ok(())
}

impl SerializeGeo for SvgPlain {
    fn write(&self, layout: &HGridLayout, out: &mut dyn io::Write) -> io::Result<()> {
        write_svg(layout, out, self.pad, false)
    }
}

impl SerializeGeo for SvgRich {
    fn write(&self, layout: &HGridLayout, out: &mut dyn io::Write) -> io::Result<()> {
        write_svg(layout, out, self.pad, true)
    }
}

fn fmt_v3(v: Vec3) -> String {
    format!("[{:.4},{:.4},{:.4}]", v.x, v.y, v.z)
}

impl SerializeGeo for JsonV1 {
    fn write(&self, layout: &HGridLayout, out: &mut dyn io::Write) -> io::Result<()> {
        let f = collect_frame(layout, 0.0);
        let hex_entries: Vec<String> = f
            .hex_data
            .iter()
            .map(|hd| {
                let corners: Vec<String> = hd
                    .corners
                    .iter()
                    .map(|(x, z)| format!("[{x:.4},{z:.4}]"))
                    .collect();
                format!(
                    r#"{{"center":[{:.4},{:.4}],"corners":[{}],"height":{:.4}}}"#,
                    hd.center.x,
                    hd.center.y,
                    corners.join(","),
                    hd.height,
                )
            })
            .collect();
        let edge_entries: Vec<String> = f
            .long_edges
            .iter()
            .map(|(a, b)| format!("[{},{}]", fmt_v3(*a), fmt_v3(*b)))
            .collect();
        let quad_entries: Vec<String> = layout
            .gap_quads()
            .iter()
            .map(|q| {
                format!(
                    "[{},{},{},{}]",
                    fmt_v3(q[0]),
                    fmt_v3(q[1]),
                    fmt_v3(q[2]),
                    fmt_v3(q[3])
                )
            })
            .collect();
        let tri_entries: Vec<String> = layout
            .gap_tris()
            .iter()
            .map(|t| format!("[{},{},{}]", fmt_v3(t[0]), fmt_v3(t[1]), fmt_v3(t[2])))
            .collect();
        writeln!(
            out,
            "{{\"hexes\":[{}],\"edges\":[{}],\"quads\":[{}],\"tris\":[{}]}}",
            hex_entries.join(","),
            edge_entries.join(","),
            quad_entries.join(","),
            tri_entries.join(","),
        )?;
        Ok(())
    }
}

impl SerializeGeo for JsonV2 {
    fn write(&self, layout: &HGridLayout, out: &mut dyn io::Write) -> io::Result<()> {
        let tri_entries: Vec<String> = layout
            .all_tris()
            .iter()
            .map(|t| format!("[{},{},{}]", fmt_v3(t[0]), fmt_v3(t[1]), fmt_v3(t[2])))
            .collect();
        writeln!(
            out,
            "{{\"version\":2,\"tris\":[{}]}}",
            tri_entries.join(","),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HGridSettings;

    fn small_layout() -> HGridLayout {
        HGridLayout::from_settings(&HGridSettings {
            radius: 2,
            ..HGridSettings::default()
        })
    }

    #[test]
    fn json_v1_has_required_fields() {
        let layout = small_layout();
        let mut buf = Vec::new();
        JsonV1.write(&layout, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"hexes\":["));
        assert!(s.contains("\"edges\":["));
        assert!(s.contains("\"quads\":["));
        assert!(s.contains("\"tris\":["));
    }

    #[test]
    fn json_v2_has_version_and_tris() {
        let layout = small_layout();
        let mut buf = Vec::new();
        JsonV2.write(&layout, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"version\":2"));
        assert!(s.contains("\"tris\":["));
        assert!(!s.contains("\"hexes\""));
        assert!(!s.contains("\"quads\""));
    }

    #[test]
    fn svg_plain_emits_svg_root() {
        let layout = small_layout();
        let mut buf = Vec::new();
        SvgPlain { pad: 0.5 }.write(&layout, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("<svg"));
        assert!(s.contains("</svg>"));
    }

    #[test]
    fn svg_rich_includes_text_labels() {
        let layout = small_layout();
        let mut buf = Vec::new();
        SvgRich { pad: 0.5 }.write(&layout, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("<text"));
    }
}
