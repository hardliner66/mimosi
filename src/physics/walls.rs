use rapier3d::prelude::*;

use crate::helper::vector3;

pub fn create_walls(physics: &mut RigidBodySet, colliders: &mut ColliderSet) {
    let wall_thickness = 0.1;
    let wall_height = 0.5;
    let maze_size = 10.0;

    let wall_positions = vec![
        (
            vector3(maze_size / 2.0, 0.0, wall_height / 2.0),
            vector3(wall_thickness, maze_size, wall_height),
        ),
        (
            vector3(-maze_size / 2.0, 0.0, wall_height / 2.0),
            vector3(wall_thickness, maze_size, wall_height),
        ),
        (
            vector3(0.0, maze_size / 2.0, wall_height / 2.0),
            vector3(maze_size, wall_thickness, wall_height),
        ),
        (
            vector3(0.0, -maze_size / 2.0, wall_height / 2.0),
            vector3(maze_size, wall_thickness, wall_height),
        ),
        (
            vector3(1.0, 0.0, wall_height / 2.0),
            vector3(0.1, 3.0, wall_height),
        ),
        (
            vector3(-1.0, 1.0, wall_height / 2.0),
            vector3(3.0, 0.1, wall_height),
        ),
    ];

    for (position, size) in wall_positions {
        let rigid_body = RigidBodyBuilder::fixed().translation(position).build();
        let collider = ColliderBuilder::cuboid(size.x, size.y, size.z)
            .friction(1.0)
            .build();

        let body_handle = physics.insert(rigid_body);
        colliders.insert_with_parent(collider, body_handle, physics);
    }
}
