# Cerebrum

> OpenWolf's learning memory. Updated automatically as the AI learns from interactions.
> Do not edit manually unless correcting an error.
> Last updated: 2026-04-10

## User Preferences

<!-- How the user likes things done. Code style, tools, patterns, communication. -->

## Key Learnings

- **Project:** hex-terrain
- **Description:** [![codecov](https://codecov.io/gh/dynnamitt/hex-terrain/graph/badge.svg)](https://codecov.io/gh/dynnamitt/hex-terrain)
- **Bevy 0.18 ShaderRef:** `ShaderRef` moved from `bevy_render` to `bevy_shader`. Import via `bevy::shader::ShaderRef`. `AsBindGroup` stays at `bevy::render::render_resource::AsBindGroup`.
- **Bevy 0.18 Material bind group:** Material uniforms use group 3 (`MATERIAL_BIND_GROUP_INDEX = 3`). In WGSL shaders, use `@group(#{MATERIAL_BIND_GROUP})` — never hardcode group index.
- **Shared material clone-on-transition:** When multiple entities share the same `Handle<StandardMaterial>`, mutating the asset via `get_mut` changes it for ALL entities. Must clone the material per-entity before animating (QuadEdge pattern).

## Do-Not-Repeat

<!-- Mistakes made and corrected. Each entry prevents the same mistake recurring. -->
<!-- Format: [YYYY-MM-DD] Description of what went wrong and what to do instead. -->

## Decision Log

<!-- Significant technical decisions with rationale. Why X was chosen over Y. -->
