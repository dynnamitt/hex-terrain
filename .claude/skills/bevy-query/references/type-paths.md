# BRP Type Paths

Component and resource types in BRP use the full Rust module path where the
type is **defined**, not where it is re-exported. Private modules are included.

> **Important:** Use `world.list` at runtime to verify paths. Bevy internal
> paths can change between versions; the paths below are for **Bevy 0.18**.

## This project (hex-terrain)

> These paths reflect the current module layout and **will change** if modules
> are renamed or types are moved. Always verify with `world.list` if unsure.

### Terrain entities (`terrain/entities.rs`)
```
hex_terrain::terrain::entities::HexGrid
hex_terrain::terrain::entities::HexSunDisc
hex_terrain::terrain::entities::HeightPole
hex_terrain::terrain::entities::QuadLeaf
hex_terrain::terrain::entities::TriLeaf
hex_terrain::terrain::entities::FlowerState
hex_terrain::terrain::entities::HexEntities
hex_terrain::terrain::entities::NeonMaterials
```

### Drone entities (`drone/entities.rs`)
```
hex_terrain::drone::entities::Player
hex_terrain::drone::entities::CursorRecentered
```

### Intro entities (`intro/entities.rs`)
```
hex_terrain::intro::entities::IntroTimer
```

### Main (`main.rs`)
```
hex_terrain::GameState
hex_terrain::PlayerPos
```

## Bevy 0.18 built-in types

### Transform
```
bevy_transform::components::transform::Transform
bevy_transform::components::global_transform::GlobalTransform
```

### Hierarchy
```
bevy_hierarchy::components::children::Children
bevy_hierarchy::components::parent::ChildOf
```

### Visibility
```
bevy_render::view::visibility::Visibility
bevy_render::view::visibility::InheritedVisibility
bevy_render::view::visibility::ViewVisibility
```

### Identity
```
bevy_core::name::Name
```

### Camera
```
bevy_render::camera::camera::Camera
bevy_render::camera::projection::OrthographicProjection
bevy_render::camera::projection::PerspectiveProjection
bevy_core_pipeline::core_3d::camera_3d::Camera3d
bevy_core_pipeline::core_2d::camera_2d::Camera2d
```

### Rendering
```
bevy_render::view::Hdr
bevy_render::mesh::components::Mesh3d
bevy_render::mesh::components::Mesh2d
bevy_pbr::components::MeshMaterial3d<bevy_pbr::StandardMaterial>
bevy_core_pipeline::tonemapping::Tonemapping
bevy_post_process::bloom::Bloom
```

### Lights
```
bevy_pbr::light::point_light::PointLight
bevy_pbr::light::directional_light::DirectionalLight
bevy_pbr::light::spot_light::SpotLight
bevy_pbr::light::ambient_light::AmbientLight
```

### Input
```
bevy_input::button_input::ButtonInput<bevy_input::keyboard::KeyCode>
bevy_input::button_input::ButtonInput<bevy_input::mouse::MouseButton>
```

### Window
```
bevy_window::window::Window
bevy_window::cursor::CursorOptions
```

### Time
```
bevy_time::time::Time
```

### UI
```
bevy_ui::ui_node::Node
bevy_ui::ui_node::Text
bevy_ui::ui_node::BackgroundColor
bevy_ui::ui_node::BorderColor
bevy_ui::widget::button::Button
bevy_ui::widget::image::ImageNode
```

### States (resources, not components)
```
bevy_state::state::State<hex_terrain::GameState>
bevy_state::state::NextState<hex_terrain::GameState>
```

Note: State resources may not be queryable via BRP `world.query` (which is
entity-scoped). Use indirect verification (e.g. checking which state-gated
entities exist) instead.

## Discovering type paths at runtime

When unsure of a type path, use `world.list` to dump all registered components:

```bash
curl -sf -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"world.list","id":0}' \
  | jq '.result[]' | grep -i "keyword"
```
