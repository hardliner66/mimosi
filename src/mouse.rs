use std::collections::HashMap;

use notan::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::{
    engine::{MouseData, SensorInfo, Sensors},
    helper::Vec2Def,
};

#[derive(Serialize, Deserialize)]
pub struct Sensor {
    #[serde(with = "Vec2Def")]
    pub position_offset: Vec2, // Offset relative to the center of the rectangle
    pub angle: f32, // Angle in radians
    #[serde(skip)]
    pub value: f32,
    #[serde(skip)]
    pub closest_point: Vec2,
}

#[derive(Serialize, Deserialize)]
pub struct MouseConfig {
    pub wheel_base: f32, // Distance between the wheels
    pub wheel_radius: f32,
    pub wheel_friction: f32,
    pub mass: f32, // Mass of the micromouse
    pub max_speed: f32,

    pub width: f32,  // Width of the mouse
    pub length: f32, // Length of the mouse (not including the triangle)

    pub encoder_resolution: usize,

    pub sensors: HashMap<String, Sensor>,
}

pub struct Micromouse {
    pub position: Vec2,
    pub width: f32,  // Width of the mouse
    pub length: f32, // Length of the mouse (not including the triangle)
    pub sensors: HashMap<String, Sensor>,

    pub wheel_friction: f32,
    pub orientation: f32, // Orientation angle in radians
    pub wheel_base: f32,  // Distance between the wheels
    pub left_power: f32,
    pub right_power: f32,
    pub left_encoder: usize,
    pub right_encoder: usize,
    pub encoder_resolution: usize,

    pub wheel_radius: f32,
    pub left_velocity: f32,  // Current velocity of the left wheels
    pub right_velocity: f32, // Current velocity of the right wheels
    pub max_speed: f32,
    pub mass: f32, // Mass of the micromouse
}

impl Micromouse {
    pub fn new(
        MouseConfig {
            wheel_base,
            wheel_radius,
            width,
            length,
            sensors,
            mass,
            max_speed,
            wheel_friction,
            encoder_resolution,
        }: MouseConfig,
        position: Vec2,
        orientation: f32,
    ) -> Self {
        Self {
            position,
            wheel_base,
            width,
            mass,
            length,
            max_speed,
            wheel_radius,
            left_encoder: 0,
            right_encoder: 0,
            encoder_resolution,
            sensors: sensors
                .into_iter()
                .map(|(n, s)| {
                    (
                        n,
                        Sensor {
                            angle: s.angle.to_radians(),
                            ..s
                        },
                    )
                })
                .collect(),
            orientation,
            wheel_friction,
            left_velocity: 0.0,
            right_velocity: 0.0,
            left_power: 0.0,
            right_power: 0.0,
        }
    }

    pub fn get_data(&self, delta_time: f32, crashed: bool) -> MouseData {
        let Micromouse {
            width,
            length,
            sensors,
            wheel_friction,
            wheel_base,
            left_power,
            right_power,
            left_encoder,
            right_encoder,
            encoder_resolution,
            mass,
            ..
        } = &self;
        MouseData {
            delta_time,
            wheel_base: *wheel_base,
            wheel_friction: *wheel_friction,
            mass: *mass,
            width: *width,
            length: *length,
            sensors: Sensors(
                sensors
                    .iter()
                    .map(|(n, v)| (n.clone(), SensorInfo::from(v)))
                    .collect(),
            ),
            left_encoder: *left_encoder,
            right_encoder: *right_encoder,
            left_power: *left_power,
            right_power: *right_power,
            encoder_resolution: *encoder_resolution,
            crashed,
        }
    }

    pub fn set_left_power(&mut self, power: f32) {
        self.left_power = power.clamp(-1.0, 1.0);
    }

    pub fn set_right_power(&mut self, power: f32) {
        self.right_power = power.clamp(-1.0, 1.0);
    }

    pub fn update_from_data(&mut self, data: MouseData) {
        self.set_left_power(data.left_power);
        self.set_right_power(data.right_power);
    }

    pub fn update(&mut self, dt: f32, maze_friction: f32) {
        // Calculate acceleration based on power input and friction
        let left_acceleration =
            self.calculate_acceleration(self.left_power, self.left_velocity, maze_friction);
        let right_acceleration =
            self.calculate_acceleration(self.right_power, self.right_velocity, maze_friction);

        // Update velocities
        self.left_velocity += left_acceleration * dt;
        self.right_velocity += right_acceleration * dt;

        // Cap velocities at max speed
        self.left_velocity = self.left_velocity.clamp(-self.max_speed, self.max_speed);
        self.right_velocity = self.right_velocity.clamp(-self.max_speed, self.max_speed);

        // Calculate average speed and turning rate
        let average_velocity = (self.left_velocity + self.right_velocity) / 2.0;
        let turning_rate = (self.left_velocity - self.right_velocity) / self.wheel_base;

        // Update orientation and position
        self.orientation += turning_rate * dt;
        self.position.x += average_velocity * self.orientation.cos() * dt;
        self.position.y += average_velocity * self.orientation.sin() * dt;

        self.update_wheel_encoders(dt);

        // Apply friction to slow down
        self.apply_friction(dt, maze_friction);
    }

    pub fn calculate_acceleration(
        &self,
        power: f32,
        current_velocity: f32,
        maze_friction: f32,
    ) -> f32 {
        // Force applied by the motor (simple model: power * max force)
        let motor_force = power * self.max_speed;

        // Frictional force
        let friction_force = (self.wheel_friction + maze_friction) * current_velocity.abs();

        // Net force = motor force - frictional force
        let net_force = motor_force - friction_force.copysign(motor_force);

        // Acceleration = net force / mass
        net_force / self.mass
    }

    pub fn apply_friction(&mut self, dt: f32, maze_friction: f32) {
        // Reduce the wheel velocities due to friction
        let friction_force = self.wheel_friction + maze_friction;

        self.left_velocity -= self.left_velocity * friction_force * dt;
        self.right_velocity -= self.right_velocity * friction_force * dt;

        // Clamp small velocities to zero to simulate stopping due to friction
        if self.left_velocity.abs() < 0.001 {
            self.left_velocity = 0.0;
        }
        if self.right_velocity.abs() < 0.001 {
            self.right_velocity = 0.0;
        }
    }

    pub fn update_wheel_encoders(&mut self, dt: f32) {
        // Calculate the distance each wheel has traveled
        let left_distance = self.left_velocity * dt;
        let right_distance = self.right_velocity * dt;

        // Calculate the number of rotations for each wheel
        let left_rotations = left_distance / (2.0 * std::f32::consts::PI * self.wheel_radius);
        let right_rotations = right_distance / (2.0 * std::f32::consts::PI * self.wheel_radius);

        // Convert rotations to encoder ticks
        let left_ticks = left_rotations * self.encoder_resolution as f32;
        let right_ticks = right_rotations * self.encoder_resolution as f32;

        // Accumulate ticks
        self.left_encoder += left_ticks as usize;
        self.right_encoder += right_ticks as usize;
    }
}
