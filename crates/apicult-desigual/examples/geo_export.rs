//! Export apicult-desigual geometry as SVG or JSON via the [`SerializeGeo`] trait.
//!
//! ```sh
//! cargo run -p apicult-desigual --example geo_export                                # plain SVG
//! cargo run -p apicult-desigual --example geo_export -- 5 2.0                       # plain SVG, custom radius/pad
//! cargo run -p apicult-desigual --example geo_export -- 5 2.0 --format svg-rich     # rich SVG
//! cargo run -p apicult-desigual --example geo_export -- 5 2.0 --format json-v1      # gen1 JSON
//! cargo run -p apicult-desigual --example geo_export -- 5 2.0 --format json-v2      # gen2 JSON (tris only)
//! ```
//!
//! Backward-compat aliases: `--rich` → `--format svg-rich`, `--json` → `--format json-v1`.

use std::io::{self, Write};

use apicult_desigual::{
    HGridLayout, HGridSettings, JsonV1, JsonV2, SerializeGeo, SvgPlain, SvgRich,
};

fn main() {
    let mut positional: Vec<String> = Vec::new();
    let mut format: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rich" => format = Some("svg-rich".into()),
            "--json" => format = Some("json-v1".into()),
            "--format" => match args.next() {
                Some(f) => format = Some(f),
                None => {
                    eprintln!("--format requires a value (svg, svg-rich, json-v1, json-v2)");
                    std::process::exit(2);
                }
            },
            _ => positional.push(arg),
        }
    }
    let radius: u32 = positional.first().and_then(|s| s.parse().ok()).unwrap_or(3);
    let pad: f32 = positional
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4.0);

    let settings = HGridSettings {
        radius,
        point_spacing: 4.0,
        ..HGridSettings::default()
    };
    let layout = HGridLayout::from_settings(&settings);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let result = match format.as_deref().unwrap_or("svg") {
        "svg" | "svg-plain" => SvgPlain { pad }.write(&layout, &mut out),
        "svg-rich" => SvgRich { pad }.write(&layout, &mut out),
        "json-v1" => JsonV1.write(&layout, &mut out),
        "json-v2" => JsonV2.write(&layout, &mut out),
        other => {
            eprintln!("unknown format '{other}' (expected svg, svg-rich, json-v1, json-v2)");
            std::process::exit(2);
        }
    };
    if let Err(e) = result.and_then(|_| out.flush()) {
        eprintln!("write failed: {e}");
        std::process::exit(1);
    }
}
