//! Shimeji mushroom cluster spawning.
//!
//! Spawns 1–3 mushrooms using shared mesh handles with transform-based
//! variation (rotation, scale, offset) — matching Blender linked-duplicate
//! clone strategy.

use bevy::prelude::*;

use crate::{FloraCfg, FloraMaterials, Shimeji, ShimejiCap, ShimejiStem};

/// Spawn a shimeji cluster at `pos`.
///
/// - `num=1`: single tall mushroom (2 mesh entities)
/// - `num=2`: tall + shorter clone (4 mesh entities)
/// - `num=3`: tall + 2 cascading shorter clones (6 mesh entities)
///
/// Returns the [`Shimeji`] root entities (not the mesh children).
pub fn shimeji_cluster(
    commands: &mut Commands,
    assets: &FloraMaterials,
    cfg: &FloraCfg,
    pos: Vec3,
    num: u8,
) -> (Entity, Vec<Entity>) {
    let num = num.clamp(1, 3);
    let mut roots = Vec::with_capacity(num as usize);

    // Cluster root (invisible pivot at world pos)
    let cluster = commands
        .spawn((
            Name::new("ShimejiCluster"),
            Transform::from_translation(pos),
            Visibility::default(),
        ))
        .id();

    // Mushroom #0: full height, identity local transform
    roots.push(spawn_one(
        commands,
        assets,
        cfg,
        cluster,
        Transform::default(),
    ));

    if num >= 2 {
        let tf = clone_tf(cfg, 1);
        roots.push(spawn_one(commands, assets, cfg, cluster, tf));
    }

    if num >= 3 {
        let tf = clone_tf(cfg, 2);
        roots.push(spawn_one(commands, assets, cfg, cluster, tf));
    }

    (cluster, roots)
}

/// Spawn one mushroom as child of `parent` with local `tf`.
fn spawn_one(
    commands: &mut Commands,
    assets: &FloraMaterials,
    cfg: &FloraCfg,
    parent: Entity,
    tf: Transform,
) -> Entity {
    let scale_y = tf.scale.y;

    // Cap counter-scale: undo parent's Y squish so dome keeps its shape
    let cap_counter = if (scale_y - 1.0).abs() > f32::EPSILON {
        1.0 / scale_y
    } else {
        1.0
    };

    let mushroom = commands
        .spawn((Shimeji, Name::new("Shimeji"), tf, Visibility::default()))
        .with_child((
            ShimejiStem,
            Mesh3d(assets.stem_mesh.clone()),
            MeshMaterial3d(assets.stem_mat.clone()),
        ))
        .with_child((
            ShimejiCap,
            Mesh3d(assets.cap_mesh.clone()),
            MeshMaterial3d(assets.cap_mat.clone()),
            Transform::from_xyz(0.0, cfg.stem_height, 0.0).with_scale(Vec3::new(
                1.0,
                cap_counter,
                1.0,
            )),
        ))
        .id();

    commands.entity(parent).add_child(mushroom);
    mushroom
}

/// Compute clone transform for the `depth`-th clone (1-indexed).
///
/// Each clone compounds: offset rotated+scaled, rotation additive,
/// scale multiplicative. depth=1 is first clone, depth=2 cascades from that.
fn clone_tf(cfg: &FloraCfg, depth: u32) -> Transform {
    let rot_single = cfg.clone_rot_y_deg.to_radians();
    let total_rot = rot_single * depth as f32;
    let total_scale_y = cfg.clone_scale_y.powi(depth as i32);
    let rot_quat = Quat::from_rotation_y(total_rot);

    // Compound offset: each step rotates the offset by the previous rotation
    let mut offset = Vec3::ZERO;
    for step in 0..depth {
        let step_rot = Quat::from_rotation_y(rot_single * step as f32);
        let step_scale = cfg.clone_scale_y.powi(step as i32);
        offset += step_rot * (cfg.clone_offset * Vec3::new(1.0, step_scale, 1.0));
    }

    Transform {
        translation: offset,
        rotation: rot_quat,
        scale: Vec3::new(1.0, total_scale_y, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_tf_depth1() {
        let cfg = FloraCfg::default();
        let tf = clone_tf(&cfg, 1);
        assert!((tf.scale.y - 0.7).abs() < 0.001);
        assert!((tf.translation - cfg.clone_offset).length() < 0.001);
    }

    #[test]
    fn clone_tf_depth2_scales_compound() {
        let cfg = FloraCfg::default();
        let tf = clone_tf(&cfg, 2);
        assert!((tf.scale.y - 0.49).abs() < 0.001);
    }
}
