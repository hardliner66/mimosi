use macroquad::prelude::*;
use rapier3d::prelude::*;

pub fn create_maze_floor(
    physics: &mut RigidBodySet,
    colliders: &mut ColliderSet,
    floor_friction: f32,
) {
    let floor_size = 10.0;
    let floor_thickness = 0.1;

    let rigid_body = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 0.0, -floor_thickness / 2.0])
        .build();
    let collider = ColliderBuilder::cuboid(floor_size, floor_size, floor_thickness)
        .friction(floor_friction)
        .build();

    let body_handle = physics.insert(rigid_body);
    colliders.insert_with_parent(collider, body_handle, physics);
}
