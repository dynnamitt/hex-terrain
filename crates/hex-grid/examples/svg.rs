//! Renders the hex grid as an SVG file to stdout.
//!
//! ```sh
//! cargo run -p hex-grid --example svg > grid.svg
//! ```

use hex_grid::{HGridLayout, HGridSettings};
use hexx::{Hex, shapes};

fn main() {
    let settings = HGridSettings {
        radius: 3,
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

    struct HexData {
        corners: [(f32, f32); 6],
    }

    let hex_data: Vec<HexData> = hexes
        .iter()
        .filter_map(|&hex| {
            let mut corners = [(0.0f32, 0.0f32); 6];
            for i in 0..6u8 {
                let v = layout.vertex(hex, i)?;
                corners[i as usize] = (v.x, v.z);
                min_x = min_x.min(v.x);
                max_x = max_x.max(v.x);
                min_z = min_z.min(v.z);
                max_z = max_z.max(v.z);
            }
            Some(HexData { corners })
        })
        .collect();

    let padding = settings.point_spacing;
    let vb_x = min_x - padding;
    let vb_z = min_z - padding;
    let vb_w = (max_x - min_x) + 2.0 * padding;
    let vb_h = (max_z - min_z) + 2.0 * padding;

    let stroke = 0.12;
    let outline = stroke * 3.0;

    // SVG header — transparent background.
    println!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb_x:.1} {vb_z:.1} {vb_w:.1} {vb_h:.1}" width="800" height="800">"##,
    );

    // ── Hex face outlines (white outline, then black stroke) ────

    // White outline pass.
    for hd in &hex_data {
        let points: String = hd
            .corners
            .iter()
            .map(|(x, z)| format!("{x:.2},{z:.2}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!(
            r##"  <polygon points="{points}" fill="none" stroke="white" stroke-width="{outline:.2}" stroke-linejoin="round"/>"##,
        );
    }
    // Black stroke pass.
    for hd in &hex_data {
        let points: String = hd
            .corners
            .iter()
            .map(|(x, z)| format!("{x:.2},{z:.2}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!(
            r##"  <polygon points="{points}" fill="none" stroke="black" stroke-width="{stroke:.2}" stroke-linejoin="round"/>"##,
        );
    }

    // ── Quad gap long edges (white outline, then black stroke) ──

    let long_edges = layout.quad_long_edges();

    // White outline pass.
    for (from, to) in &long_edges {
        println!(
            r##"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="white" stroke-width="{outline:.2}" stroke-linecap="round"/>"##,
            from.x, from.y, to.x, to.y,
        );
    }
    // Black stroke pass.
    for (from, to) in &long_edges {
        println!(
            r##"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="black" stroke-width="{stroke:.2}" stroke-linecap="round"/>"##,
            from.x, from.y, to.x, to.y,
        );
    }

    println!("</svg>");
}
