//! Renders the hex grid as an SVG file to stdout.
//!
//! ```sh
//! cargo run -p hex-grid --example svg > grid.svg
//! ```

use hex_grid::{HGridLayout, HGridSettings};
use hexx::{Hex, shapes};

fn main() {
    let settings = HGridSettings {
        radius: 7,
        point_spacing: 4.0,
        ..HGridSettings::default()
    };
    let layout = HGridLayout::from_settings(&settings);

    // Collect hex data for bounds calculation.
    let hexes: Vec<Hex> = shapes::hexagon(Hex::ZERO, settings.radius).collect();

    // World-space bounds (XZ plane, Y is height).
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut max_h: f32 = 0.0;

    struct HexData {
        center: glam::Vec2,
        corners: [(f32, f32); 6],
        height: f32,
    }

    let hex_data: Vec<HexData> = hexes
        .iter()
        .filter_map(|&hex| {
            let center = layout.hex_to_world_pos(hex);
            let height = layout.height(&hex)?;
            max_h = max_h.max(height);
            let mut corners = [(0.0f32, 0.0f32); 6];
            for i in 0..6u8 {
                let v = layout.vertex(hex, i)?;
                // v is (x, height, z) in world space; we project to (x, z).
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

    let padding = settings.point_spacing;
    let vb_x = min_x - padding;
    let vb_z = min_z - padding;
    let vb_w = (max_x - min_x) + 2.0 * padding;
    let vb_h = (max_z - min_z) + 2.0 * padding;

    // SVG header.
    println!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb_x:.1} {vb_z:.1} {vb_w:.1} {vb_h:.1}" width="800" height="800">"##,
    );
    println!(
        r##"<rect x="{vb_x:.1}" y="{vb_z:.1}" width="{vb_w:.1}" height="{vb_h:.1}" fill="#050810"/>"##,
    );

    // Draw hex faces.
    for hd in &hex_data {
        let t = if max_h > 0.0 { hd.height / max_h } else { 0.0 };
        // Height-based color ramp: dark olive → bright green.
        let r = (40.0 + t * 80.0) as u8;
        let g = (60.0 + t * 180.0) as u8;
        let b = (30.0 + t * 40.0) as u8;

        let points: String = hd
            .corners
            .iter()
            .map(|(x, z)| format!("{x:.2},{z:.2}"))
            .collect::<Vec<_>>()
            .join(" ");

        println!(
            r##"  <polygon points="{points}" fill="rgb({r},{g},{b})" stroke="#1a3a1a" stroke-width="0.08" opacity="0.9"/>"##,
        );
    }

    // Draw quad gap long edges (hex↔neighbor bridging lines, no fill).
    let long_edges = layout.quad_long_edges();
    for (from, to) in &long_edges {
        println!(
            r##"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#40e0d0" stroke-width="0.06" opacity="0.7"/>"##,
            from.x, from.y, to.x, to.y,
        );
    }

    // Draw height labels at hex centers.
    let font = settings.point_spacing * 0.22;
    for hd in &hex_data {
        println!(
            r##"  <text x="{:.2}" y="{:.2}" font-size="{font:.2}" fill="#aaa" text-anchor="middle" dominant-baseline="central">{:.1}</text>"##,
            hd.center.x, hd.center.y, hd.height,
        );
    }

    println!("</svg>");
}
