# Updates

## v0.0.7

- Shimeji mushroom clusters sprout on hex faces — procedural 6-sided stems with domed caps via new `flora` crate
- FoV overlay rewritten as WGSL shader (ExtendedMaterial) — shape-matching band patterns, cyan glow, and shader-drawn aim star replace per-entity material swaps
- Lazy FovMaterial — only hexes in the FoV ring run the custom shader, rest stay on plain StandardMaterial
- Biome system with `--biome` CLI flag — standard, rocky, mesa, and mesa-lush presets control mineral distribution and flora frequency
- Feature-gated `remote` and `inspector` to trim default binary size

## v0.0.6

- Procedural atmosphere sky replaces flat twilight background — Hillaire 2020 scattering with IBL fill eliminates pale gap artifacts on camera rotation
- Computed gap surface normals (default on) — gap faces respond to terrain slope lighting instead of fixed Y-up
- hex-grid workspace crate — layout, math, and gap geometry extracted for reuse with SVG example
- Matte mineral surfaces — low reflectance (0.1) removes specular flash on dark minerals like Basalt and Obsidian
- WASM fallback — AmbientLight replaces atmosphere IBL on WebGL2 where compute shaders are unavailable

## v0.0.5

- Scene lighting overhaul — DirectionalLight + AmbientLight replace pure-emissive model, hex/gap faces now PBR-shaded
- Selective edge bloom — only FoV edges glow via emissive, non-FoV edges show muted cyan outlines
- Laser fire swaps aim-stars to yellow glow and hex face to stepped radial gradient texture
- Deep twilight blue sky replaces near-black background
- MeshRayCast entity filtering narrows aim targeting to hex faces and aim-stars only

## v0.0.4

- Ore extraction lowers hex faces and realigns gap geometry on both owner and neighbor sides
- Aim-star lines highlight the targeted hex face with rotating cuboid overlays
- Stable InSight targeting — no more flicker when crosshair sits on aim-star edges
- Intro animation refactored into drone's AnimationGraph with smoother CubicInOut easing
- Release notes auto-injected from UPDATES.md into the loading overlay at deploy time

## v0.0.3

- Laser pipe snaps to aimed hex face — points at InSight target in real-time
- Raycast-based ground level replaces interpolation — camera rides actual mesh surfaces
- Asymmetric height lerp — instant snap up on rising terrain, smooth ease down on descent

## v0.0.2

- Laser-drill aiming system with animated pipe pivot and ray visuals
- FoV-aware color transitions — hex faces, gaps, and edges smoothly fade between palettes
- Crosshair overlay and InSight targeting for the hex under screen center
- Drone altitude now lerps relative to terrain height for smooth ground-following
- Animated intro sequence with tilt-up camera reveal and bloom post-processing

## v0.0.1

- First WASM build
- Hello drone-world!
