use crate::{helper::vector3, micromouse::sensor::Sensor};

use macroquad::prelude::*;
use rapier3d::prelude::*;

pub struct Micromouse {
    pub body_handle: RigidBodyHandle,
    pub left_wheel_power: f32,
    pub right_wheel_power: f32,
    pub sensors: Vec<Sensor>,
}

impl Micromouse {
    pub fn new(
        physics: &mut RigidBodySet,
        colliders: &mut ColliderSet,
        floor_friction: f32,
    ) -> Self {
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.05]) // Slightly above the ground to avoid initial collision
            .build();
        let body_handle = physics.insert(rigid_body);

        let shape = ColliderBuilder::cuboid(0.1, 0.2, 0.05) // 3D shape for the micromouse
            .friction(floor_friction) // Friction between mouse wheels and floor
            .build();
        colliders.insert_with_parent(shape, body_handle, physics);

        // Configurable sensors
        let sensors = vec![
            Sensor {
                position_offset: vector3(0.1, 0.1, 0.0),
                angle_offset: 0.0,
            },
            Sensor {
                position_offset: vector3(0.1, -0.1, 0.0),
                angle_offset: 0.0,
            },
            Sensor {
                position_offset: vector3(0.0, 0.2, 0.0),
                angle_offset: 90.0,
            },
            Sensor {
                position_offset: vector3(0.0, -0.2, 0.0),
                angle_offset: -90.0,
            },
            Sensor {
                position_offset: vector3(-0.1, 0.1, 0.0),
                angle_offset: 180.0,
            },
            Sensor {
                position_offset: vector3(-0.1, -0.1, 0.0),
                angle_offset: 180.0,
            },
        ];

        Micromouse {
            body_handle,
            left_wheel_power: 0.0,
            right_wheel_power: 0.0,
            sensors,
        }
    }

    pub fn handle_input(&mut self) {
        if is_key_down(KeyCode::Up) {
            self.left_wheel_power = 1.0;
            self.right_wheel_power = 1.0;
        } else if is_key_down(KeyCode::Down) {
            self.left_wheel_power = -1.0;
            self.right_wheel_power = -1.0;
        } else if is_key_down(KeyCode::Left) {
            self.left_wheel_power = -1.0;
            self.right_wheel_power = 1.0;
        } else if is_key_down(KeyCode::Right) {
            self.left_wheel_power = 1.0;
            self.right_wheel_power = -1.0;
        } else {
            self.left_wheel_power = 0.0;
            self.right_wheel_power = 0.0;
        }
    }

    pub fn apply_motor_forces(&mut self, physics: &mut RigidBodySet) {
        let body = &mut physics[self.body_handle];
        let rotation = body.rotation();

        let motor_strength = 5.0; // Strength of the motor

        // Calculate the forward direction
        let forward = rotation.transform_vector(&vector![1.0, 0.0, 0.0]);

        // Calculate the left and right wheel forces
        let left_force = self.left_wheel_power * motor_strength;
        let right_force = self.right_wheel_power * motor_strength;

        // Calculate the resulting force and torque
        let forward_force = (left_force + right_force) * forward;
        let torque = vector![0.0, 0.0, (right_force - left_force) * 0.05];

        // Apply forces and torque
        body.add_force(forward_force, true);
        body.apply_torque_impulse(torque, true);
    }

    pub fn sense_walls(
        &self,
        physics: &RigidBodySet,
        query_pipeline: &QueryPipeline,
        colliders: &ColliderSet,
    ) -> Vec<(Vec3, Vec3)> {
        let body = &physics[self.body_handle];
        let position = body.translation();
        let rotation = body.rotation();

        let max_distance = 5.0;
        let mut rays = Vec::new();

        // for sensor in &self.sensors {
        //     let sensor_world_pos = position + rotation * sensor.position_offset;
        //     let sensor_dir: Vec3 = rotation
        //         * Quat::from_rotation_z(sensor.angle_offset.to_radians()).into()
        //         * vec3(1.0, 0.0, 0.0);
        //     let ray = Ray::new(
        //         point![sensor_world_pos.x, sensor_world_pos.y, sensor_world_pos.z],
        //         vector![sensor_dir.x, sensor_dir.y, sensor_dir.z],
        //     );
        //     let query_filter = QueryFilter::new();
        //     if let Some(hit) =
        //         query_pipeline.cast_ray(physics, &colliders, &ray, max_distance, true, query_filter)
        //     {
        //         let hit_point = ray.point_at(hit.1);
        //         rays.push((
        //             vec3(sensor_world_pos.x, sensor_world_pos.y, sensor_world_pos.z),
        //             vec3(hit_point.x, hit_point.y, hit_point.z),
        //         ));
        //     }
        // }

        rays
    }

    pub fn draw(
        &self,
        physics: &RigidBodySet,
        query_pipeline: &QueryPipeline,
        colliders: &ColliderSet,
    ) {
        let body = &physics[self.body_handle];
        let position = body.translation();

        draw_cube(
            vec3(position.x, position.y, position.z),
            vec3(0.1, 0.2, 0.05),
            None,
            BLUE,
        );

        let rays = self.sense_walls(physics, query_pipeline, colliders);
        for (start, end) in rays {
            draw_line_3d(start, end, RED);
        }
    }
}
