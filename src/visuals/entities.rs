use bevy::prelude::*;

/// Shared material handles for the neon visual theme.
#[derive(Resource)]
pub struct ActiveNeonMaterials {
    /// Bright emissive cyan used for edge lines.
    pub edge_material: Handle<StandardMaterial>,
    /// Dark surface material applied to hex face meshes.
    pub hex_face_material: Handle<StandardMaterial>,
    /// Slightly warm dark material for gap-fill quads and triangles.
    pub gap_face_material: Handle<StandardMaterial>,
}
