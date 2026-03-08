use bevy::prelude::*;

#[derive(Resource)]
pub struct DroneMaterials {
    pub pipe: Handle<StandardMaterial>,
    pub laser_ray: Handle<StandardMaterial>,
}

impl DroneMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            pipe: materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.15, 0.15), // dark gray
                metallic: 0.9,
                perceptual_roughness: 0.3,
                ..default()
            }),
            laser_ray: materials.add(StandardMaterial {
                base_color: Color::BLACK,
                emissive: LinearRgba::new(10.0, 0.0, 0.0, 1.0), // intense red bloom
                ..default()
            }),
        }
    }
}
