//! Hex grid geometry: layout, noise-driven terrain, and pure math helpers.
//!
//! Zero ECS dependencies — uses `glam` for vector math, `hexx` for hex
//! coordinates, and `noise` for terrain generation.

pub mod layout;
pub mod math;

pub use layout::{HGridLayout, HGridSettings};
pub use math::{
    edge_cuboid_transform, gap_filler, gap_vertex_data, idw_interpolate_height, map_noise_to_range,
    quad_corner_indices,
};
