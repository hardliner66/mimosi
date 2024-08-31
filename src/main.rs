use std::{
    fmt::Debug,
    ops::Deref,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use macroquad::prelude::*;
use rhai::{
    packages::{CorePackage, Package},
    CustomType, Engine, Scope, TypeBuilder, AST,
};
use serde::{Deserialize, Serialize};

const RIGHT: f32 = 0.0;
// const UP_RIGHT: f32 = std::f32::consts::FRAC_PI_4;
const UP: f32 = std::f32::consts::FRAC_PI_2;
// const UP_LEFT: f32 = UP + std::f32::consts::FRAC_PI_4;
const LEFT: f32 = std::f32::consts::PI;
// const DOWN_LEFT: f32 = LEFT + std::f32::consts::FRAC_PI_4;
const DOWN: f32 = 3.0 * std::f32::consts::FRAC_PI_2;
// const DOWN_RIGHT: f32 = DOWN + std::f32::consts::FRAC_PI_4;

pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(128, 64);

    let package = CorePackage::new();

    // Register the package into the 'Engine' by converting it into a shared module.
    engine.register_global_module(package.as_shared_module());

    engine
        .build_type::<MouseData>()
        .register_fn("to_debug", |d: MouseData| format!("{d:#?}"))
        .build_type::<SensorInfo>()
        .build_type::<Sensors>()
        .register_iterator::<Sensors>()
        .register_indexer_get(Sensors::get_sensors);

    engine
}

#[derive(Debug, Clone, Copy)]
struct Ray {
    origin: Vec2,
    direction: Vec2,
}

impl Ray {
    fn intersect(&self, wall: &Wall) -> Option<Vec2> {
        let edges = [
            (wall.p1, wall.p2),
            (wall.p2, wall.p3),
            (wall.p3, wall.p4),
            (wall.p4, wall.p1),
        ];

        let mut found = None;

        for (p1, p2) in edges {
            let wall_dir = p2 - p1;
            let perp_wall_dir = wall_dir.perp();

            let ray_to_wall_start = p1 - self.origin;

            let denom = self.direction.dot(perp_wall_dir);

            if denom.abs() < std::f32::EPSILON {
                continue;
            }

            let t1 = ray_to_wall_start.dot(perp_wall_dir) / denom;
            let t2 = ray_to_wall_start.dot(self.direction.perp()) / denom;

            if t1 >= 0.0 && t2 >= 0.0 && t2 <= 1.0 {
                found = Some(Vec2 {
                    x: self.origin.x + t1 * self.direction.x,
                    y: self.origin.y + t1 * self.direction.y,
                });
            }
        }
        found
    }
    fn find_nearest_intersection(&self, walls: &[Wall]) -> Option<(Vec2, f32)> {
        let mut nearest_intersection: Option<Vec2> = None;
        let mut nearest_distance = f32::MAX;

        for wall in walls {
            if let Some(intersection) = self.intersect(wall) {
                let distance = (intersection.x - self.origin.x).powi(2)
                    + (intersection.y - self.origin.y).powi(2);

                if distance < nearest_distance {
                    nearest_distance = distance;
                    nearest_intersection = Some(intersection);
                }
            }
        }

        nearest_intersection.map(|i| (i, nearest_distance))
    }
}

#[derive(Serialize, Deserialize)]
struct Sensor {
    #[serde(with = "Vec2Def")]
    position_offset: Vec2, // Offset relative to the center of the rectangle
    angle: f32, // Angle in radians
    #[serde(skip)]
    value: f32,
    #[serde(skip)]
    closest_point: Vec2,
}

#[derive(Clone, CustomType, Debug, Default)]
struct SensorInfo {
    #[rhai_type(readonly)]
    position_offset: Vec2, // Offset relative to the center of the rectangle
    #[rhai_type(readonly)]
    angle: f32, // Angle in radians
    #[rhai_type(readonly)]
    value: f32,
}

impl From<&Sensor> for SensorInfo {
    fn from(
        &Sensor {
            position_offset,
            angle,
            value,
            ..
        }: &Sensor,
    ) -> Self {
        Self {
            position_offset,
            angle: angle.to_degrees(),
            value,
        }
    }
}

impl From<Sensor> for SensorInfo {
    fn from(sensor: Sensor) -> Self {
        (&sensor).into()
    }
}

#[derive(Clone, CustomType, Debug)]
struct Sensors(#[rhai_type(skip)] Vec<SensorInfo>);

impl IntoIterator for Sensors {
    type Item = SensorInfo;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Sensors {
    fn get_sensors(&mut self, index: i64) -> SensorInfo {
        self.0[index as usize].clone()
    }
}

#[derive(Clone, CustomType, Debug)]
struct MouseData {
    #[rhai_type(readonly)]
    wheel_base: f32, // Distance between the wheels
    #[rhai_type(readonly)]
    wheel_radius: f32, // Radius of the wheels
    #[rhai_type(readonly)]
    ticks_per_revolution: usize, // Encoder resolution (ticks per full wheel revolution)
    #[rhai_type(readonly)]
    max_rpm: f32, // Maximum RPM of the motor
    #[rhai_type(readonly)]
    motor_torque: f32, // Torque provided by the motor
    #[rhai_type(readonly)]
    wheel_inertia: f32, // Rotational inertia of the wheel
    #[rhai_type(readonly)]
    tire_friction: f32,
    #[rhai_type(readonly)]
    mass: f32, // Mass of the micromouse

    #[rhai_type(readonly)]
    left_encoder: usize, // Left wheel encoder tick count
    #[rhai_type(readonly)]
    right_encoder: usize, // Right wheel encoder tick count
    #[rhai_type(readonly)]
    angular_velocity: f32,

    #[rhai_type(readonly)]
    crashed: bool,

    #[rhai_type(readonly)]
    delta_time: f32,

    #[rhai_type(readonly)]
    width: f32, // Width of the mouse
    #[rhai_type(readonly)]
    length: f32, // Length of the mouse (not including the triangle)
    #[rhai_type(readonly)]
    sensors: Sensors,

    #[rhai_type(set=MouseData::set_left_power, get=MouseData::get_left_power)]
    left_power: f32,

    #[rhai_type(set=MouseData::set_right_power, get=MouseData::get_right_power)]
    right_power: f32,
}

impl MouseData {
    fn set_left_power(&mut self, power: f32) {
        self.left_power = power.clamp(-1.0, 1.0);
    }

    fn get_left_power(&self) -> f32 {
        self.left_power
    }

    fn set_right_power(&mut self, power: f32) {
        self.right_power = power.clamp(-1.0, 1.0);
    }

    fn get_right_power(&self) -> f32 {
        self.right_power
    }
}

#[derive(Serialize, Deserialize)]
struct MouseConfig {
    wheel_base: f32,             // Distance between the wheels
    wheel_radius: f32,           // Radius of the wheels
    ticks_per_revolution: usize, // Encoder resolution (ticks per full wheel revolution)
    max_rpm: f32,                // Maximum RPM of the motor
    motor_torque: f32,           // Torque provided by the motor
    wheel_inertia: f32,          // Rotational inertia of the wheel
    tire_friction: f32,
    mass: f32, // Mass of the micromouse

    width: f32,           // Width of the mouse
    length: f32,          // Length of the mouse (not including the triangle)
    gyroscope_bias: f32,  // Bias in the gyroscope sensor (to simulate real-world imperfections)
    gyroscope_noise: f32, // Random noise in the gyroscope sensor (to simulate real-world imperfections)

    sensors: Vec<Sensor>,
}

struct Micromouse {
    position: Vec2,
    width: f32,  // Width of the mouse
    length: f32, // Length of the mouse (not including the triangle)
    sensors: Vec<Sensor>,

    tire_friction: f32,
    orientation: f32,            // Orientation angle in radians
    left_encoder: usize,         // Left wheel encoder tick count
    right_encoder: usize,        // Right wheel encoder tick count
    left_rpm: f32,               // Current RPM of the left motor
    right_rpm: f32,              // Current RPM of the right motor
    wheel_base: f32,             // Distance between the wheels
    wheel_radius: f32,           // Radius of the wheels
    ticks_per_revolution: usize, // Encoder resolution (ticks per full wheel revolution)
    max_rpm: f32,                // Maximum RPM of the motor
    motor_torque: f32,           // Torque provided by the motor
    wheel_inertia: f32,          // Rotational inertia of the wheel
    left_power: f32,
    right_power: f32,
    mass: f32,            // Mass of the micromouse
    gyroscope_bias: f32,  // Bias in the gyroscope sensor (to simulate real-world imperfections)
    gyroscope_noise: f32, // Random noise in the gyroscope sensor (to simulate real-world imperfections)
}

impl Micromouse {
    fn new(
        MouseConfig {
            wheel_base,
            width,
            length,
            sensors,
            wheel_radius,
            ticks_per_revolution,
            max_rpm,
            motor_torque,
            wheel_inertia,
            mass,
            tire_friction,
            gyroscope_bias, // Bias in the gyroscope sensor (to simulate real-world imperfections)
            gyroscope_noise, // Random noise in the gyroscope sensor (to simulate real-world imperfections)
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
            sensors,
            orientation,
            wheel_radius,
            ticks_per_revolution,
            tire_friction,
            max_rpm,
            motor_torque,
            wheel_inertia,
            gyroscope_bias,
            gyroscope_noise,
            left_encoder: 0,
            right_encoder: 0,
            left_rpm: 0.0,
            right_rpm: 0.0,
            left_power: 0.0,
            right_power: 0.0,
        }
    }

    fn get_data(&self, delta_time: f32, crashed: bool) -> MouseData {
        let Micromouse {
            width,
            length,
            sensors,
            tire_friction,
            wheel_base,
            wheel_radius,
            ticks_per_revolution,
            max_rpm,
            motor_torque,
            wheel_inertia,
            left_power,
            right_power,
            mass,
            left_encoder,
            right_encoder,
            ..
        } = &self;
        MouseData {
            delta_time,
            wheel_base: *wheel_base,
            wheel_radius: *wheel_radius,
            ticks_per_revolution: *ticks_per_revolution,
            max_rpm: *max_rpm,
            motor_torque: *motor_torque,
            wheel_inertia: *wheel_inertia,
            tire_friction: *tire_friction,
            mass: *mass,
            width: *width,
            length: *length,
            sensors: Sensors(sensors.iter().map(Into::into).collect()),
            left_power: *left_power,
            right_power: *right_power,
            left_encoder: *left_encoder,
            right_encoder: *right_encoder,
            crashed,
            angular_velocity: self.get_gyroscope_data(),
        }
    }

    fn get_gyroscope_data(&self) -> f32 {
        // Calculate the angular velocity (rad/s) from the difference in wheel speeds
        let angular_velocity =
            (self.right_rpm - self.left_rpm) / 60.0 * 2.0 * std::f32::consts::PI / self.wheel_base;

        // Add gyroscope bias and noise to simulate a real gyroscope
        let noisy_angular_velocity = angular_velocity
            + self.gyroscope_bias
            + (self.gyroscope_noise * ::rand::random::<f32>());

        noisy_angular_velocity
    }

    fn set_left_power(&mut self, power: f32) {
        self.left_power = power.clamp(-1.0, 1.0);
    }

    fn set_right_power(&mut self, power: f32) {
        self.right_power = power.clamp(-1.0, 1.0);
    }

    fn update_from_data(&mut self, data: MouseData) {
        self.set_left_power(data.left_power);
        self.set_right_power(data.right_power);
    }

    fn update(&mut self, dt: f32, maze_friction: f32) {
        // Constants
        let torque_constant = self.motor_torque; // Proportional constant for torque based on power
        let max_angular_velocity = (self.max_rpm / 60.0) * 2.0 * std::f32::consts::PI;

        // Combine tire friction and maze friction
        let combined_friction = self.tire_friction * maze_friction;

        // Calculate the effective torque considering combined friction and mass
        let effective_torque = |power: f32| -> f32 {
            let torque = power * torque_constant * combined_friction;
            torque / self.mass // Adjust torque by mass to simulate inertia
        };

        // Calculate torque applied by each motor
        let left_torque = effective_torque(self.left_power);
        let right_torque = effective_torque(self.right_power);

        // Calculate angular acceleration for each wheel considering inertia
        let left_angular_acceleration = left_torque / self.wheel_inertia;
        let right_angular_acceleration = right_torque / self.wheel_inertia;

        // Update RPM based on angular acceleration and delta time
        self.left_rpm += left_angular_acceleration * dt * 60.0 / (2.0 * std::f32::consts::PI);
        self.right_rpm += right_angular_acceleration * dt * 60.0 / (2.0 * std::f32::consts::PI);

        // Clamp RPM to max RPM
        self.left_rpm = self.left_rpm.clamp(-self.max_rpm, self.max_rpm);
        self.right_rpm = self.right_rpm.clamp(-self.max_rpm, self.max_rpm);

        // Calculate actual angular velocity considering the clamped RPM
        let left_angular_velocity = (self.left_rpm / 60.0) * 2.0 * std::f32::consts::PI;
        let right_angular_velocity = (self.right_rpm / 60.0) * 2.0 * std::f32::consts::PI;

        // Ensure the angular velocity does not exceed max_angular_velocity
        let left_angular_velocity =
            left_angular_velocity.clamp(-max_angular_velocity, max_angular_velocity);
        let right_angular_velocity =
            right_angular_velocity.clamp(-max_angular_velocity, max_angular_velocity);

        // Calculate the linear speeds of the wheels considering combined friction
        let left_speed = left_angular_velocity * self.wheel_radius * combined_friction;
        let right_speed = right_angular_velocity * self.wheel_radius * combined_friction;

        // Calculate the distance each wheel has traveled in this time step
        let left_distance = left_speed * dt;
        let right_distance = right_speed * dt;

        // Calculate the change in orientation
        let delta_orientation = (right_distance - left_distance) / self.wheel_base;

        // Update orientation
        self.orientation += delta_orientation;

        // Calculate the average distance traveled by the micromouse
        let distance = (left_distance + right_distance) / 2.0;

        // Update position considering the orientation
        self.position.x += distance * self.orientation.cos();
        self.position.y += distance * self.orientation.sin();

        // Convert distance traveled to encoder ticks
        let left_ticks = (left_distance / (2.0 * std::f32::consts::PI * self.wheel_radius)
            * self.ticks_per_revolution as f32) as usize;
        let right_ticks = (right_distance / (2.0 * std::f32::consts::PI * self.wheel_radius)
            * self.ticks_per_revolution as f32) as usize;

        // Update encoder counts
        self.left_encoder += left_ticks;
        self.right_encoder += right_ticks;
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum StartDirection {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Serialize, Deserialize, Debug)]
struct Wall(Rectangle);

impl Deref for Wall {
    type Target = Rectangle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Vec2")]
struct Vec2Def {
    x: f32,
    y: f32,
}

#[derive(Default, Serialize, Deserialize, Debug)]
struct Rectangle {
    #[serde(with = "Vec2Def")]
    p1: Vec2,
    #[serde(with = "Vec2Def")]
    p2: Vec2,
    #[serde(with = "Vec2Def")]
    p3: Vec2,
    #[serde(with = "Vec2Def")]
    p4: Vec2,
}

impl From<Rectangle> for Wall {
    fn from(value: Rectangle) -> Self {
        Wall(value)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Maze {
    walls: Vec<Wall>, // 2D grid representing walls in each cell
    friction: f32,    // Friction coefficient of the maze surface
    #[serde(with = "Vec2Def")]
    start: Vec2,
    start_direction: StartDirection,
    finish: Rectangle,
}

// Function to check if two line segments intersect
fn lines_intersect(p1: Vec2, p2: Vec2, q1: Vec2, q2: Vec2) -> bool {
    fn orientation(a: Vec2, b: Vec2, c: Vec2) -> i32 {
        let val = (b.y - a.y) * (c.x - b.x) - (b.x - a.x) * (c.y - b.y);
        if val == 0.0 {
            return 0;
        }
        if val > 0.0 {
            1
        } else {
            -1
        }
    }

    let o1 = orientation(p1, p2, q1);
    let o2 = orientation(p1, p2, q2);
    let o3 = orientation(q1, q2, p1);
    let o4 = orientation(q1, q2, p2);

    if o1 != o2 && o3 != o4 {
        return true;
    }

    false
}

fn rectangle_wall_collision(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2, wall: &Wall) -> bool {
    let rect_edges = [
        (p1, p2), // Top edge
        (p2, p3), // Right edge
        (p3, p4), // Bottom edge
        (p4, p1), // Left edge
    ];

    // Check each edge of the rectangle against the wall
    for &(p1, p2) in &rect_edges {
        if lines_intersect(p1, p2, wall.p1, wall.p2)
            || lines_intersect(p1, p2, wall.p2, wall.p3)
            || lines_intersect(p1, p2, wall.p3, wall.p4)
            || lines_intersect(p1, p2, wall.p4, wall.p1)
        {
            return true;
        }
    }

    false
}

fn triangle_wall_collision(a: Vec2, b: Vec2, c: Vec2, wall: &Wall) -> bool {
    let triangle_edges = [(a, b), (b, c), (c, a)];

    // Check each edge of the triangle against the wall
    for &(p1, p2) in &triangle_edges {
        if lines_intersect(p1, p2, wall.p1, wall.p2)
            || lines_intersect(p1, p2, wall.p2, wall.p3)
            || lines_intersect(p1, p2, wall.p3, wall.p4)
            || lines_intersect(p1, p2, wall.p4, wall.p1)
        {
            return true;
        }
    }

    false
}

struct Simulation {
    engine: Engine,
    mouse: Micromouse,
    collided: bool,
    finished: bool,
    maze: Maze,
    time_scale: f32, // Speed factor for the simulation and replay
    ast: AST,
}

impl Simulation {
    fn new<P: AsRef<Path>>(script: P, maze: Maze, mouse_config: MouseConfig) -> Self {
        let engine = build_engine();
        let ast = engine.compile_file(script.as_ref().to_path_buf()).unwrap();
        Self {
            mouse: Micromouse::new(
                mouse_config,
                maze.start,
                match maze.start_direction {
                    StartDirection::Up => UP,
                    StartDirection::Right => RIGHT,
                    StartDirection::Down => DOWN,
                    StartDirection::Left => LEFT,
                },
            ),
            collided: false,
            finished: false,
            maze,
            time_scale: 1.0,
            engine,
            ast,
        }
    }

    fn update(&mut self, dt: f32) {
        let dt_scaled = dt * self.time_scale;

        self.mouse.update(dt_scaled, self.maze.friction);

        for sensor in &mut self.mouse.sensors {
            let p = self.mouse.position
                + sensor
                    .position_offset
                    .rotate(Vec2::from_angle(self.mouse.orientation));
            let angle = self.mouse.orientation + sensor.angle;
            let r = Ray {
                origin: p,
                direction: Vec2::from_angle(angle),
            };
            if let Some((p, v)) = r.find_nearest_intersection(&self.maze.walls) {
                sensor.value = v;
                sensor.closest_point = p;
            }
        }

        if self.check_collisions() {
            self.collided = true;
        }

        if self.mouse.position.x >= self.maze.finish.p1.x
            && self.mouse.position.y >= self.maze.finish.p1.y
            && self.mouse.position.x <= self.maze.finish.p3.x
            && self.mouse.position.y <= self.maze.finish.p3.y
        {
            self.finished = true;
        }
    }

    fn check_collisions(&self) -> bool {
        let mouse = &self.mouse;

        let half_width = mouse.width / 2.0;
        let half_length = mouse.length / 2.0;

        // Calculate the corners of the rectangle
        let rear_left = mouse.position
            + vec2(-half_length, -half_width).rotate(Vec2::from_angle(mouse.orientation));
        let rear_right = mouse.position
            + vec2(-half_length, half_width).rotate(Vec2::from_angle(mouse.orientation));
        let front_left = mouse.position
            + vec2(half_length, -half_width).rotate(Vec2::from_angle(mouse.orientation));
        let front_right = mouse.position
            + vec2(half_length, half_width).rotate(Vec2::from_angle(mouse.orientation));
        let front_center = mouse.position
            + vec2(half_length + half_width, 0.0).rotate(Vec2::from_angle(mouse.orientation));

        let r1 = rear_left;
        let r2 = front_left;
        let r3 = front_right;
        let r4 = rear_right;

        // Draw the triangular front
        let t1 = front_left;
        let t2 = front_right;
        let t3 = front_center;

        for wall in &self.maze.walls {
            if rectangle_wall_collision(r1, r2, r3, r4, wall)
                || triangle_wall_collision(t1, t2, t3, wall)
            {
                return true;
            }
        }
        return false;
    }

    fn render(&self) {
        clear_background(LIGHTGRAY);

        // Render the maze with internal and outside walls
        self.render_maze();

        // Render the mouse
        self.render_mouse();
    }

    fn render_maze(&self) {
        for wall in &self.maze.walls {
            draw_line(
                wall.p1.x + 5.0,
                wall.p1.y + 5.0,
                wall.p2.x + 5.0,
                wall.p2.y + 5.0,
                1.0,
                BLACK,
            );
            draw_line(
                wall.p2.x + 5.0,
                wall.p2.y + 5.0,
                wall.p3.x + 5.0,
                wall.p3.y + 5.0,
                1.0,
                BLACK,
            );
            draw_line(
                wall.p3.x + 5.0,
                wall.p3.y + 5.0,
                wall.p4.x + 5.0,
                wall.p4.y + 5.0,
                1.0,
                BLACK,
            );
            draw_line(
                wall.p4.x + 5.0,
                wall.p4.y + 5.0,
                wall.p1.x + 5.0,
                wall.p1.y + 5.0,
                1.0,
                BLACK,
            );

            draw_rectangle_lines(
                self.maze.finish.p1.x + 5.0,
                self.maze.finish.p1.y + 5.0,
                self.maze.finish.p3.x - self.maze.finish.p1.x,
                self.maze.finish.p3.y - self.maze.finish.p1.y,
                2.0,
                GREEN,
            );
        }
    }

    fn render_mouse(&self) {
        let offset = vec2(5.0, 5.0);
        let mouse = &self.mouse;
        let half_width = mouse.width / 2.0;
        let half_length = mouse.length / 2.0;

        // Calculate the corners of the rectangle
        let rear_left = mouse.position
            + vec2(-half_length, -half_width).rotate(Vec2::from_angle(mouse.orientation));
        let rear_right = mouse.position
            + vec2(-half_length, half_width).rotate(Vec2::from_angle(mouse.orientation));
        let front_left = mouse.position
            + vec2(half_length, -half_width).rotate(Vec2::from_angle(mouse.orientation));
        let front_right = mouse.position
            + vec2(half_length, half_width).rotate(Vec2::from_angle(mouse.orientation));
        let front_center = mouse.position
            + vec2(half_length + half_width, 0.0).rotate(Vec2::from_angle(mouse.orientation));

        // Draw the rectangle part of the mouse
        draw_triangle(
            rear_left + offset,
            rear_right + offset,
            front_right + offset,
            RED,
        );
        draw_triangle(
            rear_left + offset,
            front_left + offset,
            front_right + offset,
            RED,
        );

        // Draw the triangular front
        draw_triangle(
            front_left + offset,
            front_right + offset,
            front_center + offset,
            BLUE,
        );

        for sensor in &self.mouse.sensors {
            let p1 = self.mouse.position
                + sensor
                    .position_offset
                    .rotate(Vec2::from_angle(mouse.orientation));
            let p2 = sensor.closest_point;
            draw_line(
                p1.x + 5.0,
                p1.y + 5.0,
                p2.x + 5.0,
                p2.y + 5.0,
                2.0,
                DARKPURPLE,
            );
        }

        if self.collided {
            draw_line(
                rear_left.x + 5.0,
                rear_left.y + 5.0,
                front_right.x + 5.0,
                front_right.y + 5.0,
                2.0,
                BLACK,
            );
            draw_line(
                rear_right.x + 5.0,
                rear_right.y + 5.0,
                front_left.x + 5.0,
                front_left.y + 5.0,
                2.0,
                BLACK,
            );
        } else if self.finished {
            draw_line(
                rear_left.x + 5.0,
                rear_left.y + 5.0,
                front_right.x + 5.0,
                front_right.y + 5.0,
                2.0,
                GREEN,
            );
            draw_line(
                rear_right.x + 5.0,
                rear_right.y + 5.0,
                front_left.x + 5.0,
                front_left.y + 5.0,
                2.0,
                GREEN,
            );
        }
    }
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    ExampleMouse,
    ExampleMaze,
    ExampleScript,
    Simulate {
        maze: PathBuf,
        mouse: PathBuf,
        script: PathBuf,
    },
}

fn parse_maze(s: &str) -> Maze {
    let mut friction = 1.0;
    let mut start = vec2(0.0, 0.0);
    let mut start_direction = StartDirection::Right;
    let mut walls = Vec::new();
    let mut finish = Rectangle::default();

    for line in s.lines() {
        if let Some((left, right)) = line.split_once(":") {
            let left = left.trim().to_uppercase();
            match left.as_str() {
                "SP" => {
                    if let Some((left, right)) = right.split_once(",") {
                        start = vec2(left.trim().parse().unwrap(), right.parse().unwrap()) * 50.0
                            + vec2(25.0, 25.0);
                    }
                }
                "SD" => {
                    start_direction = match right.trim().to_uppercase().as_str() {
                        "L" => StartDirection::Left,
                        "U" => StartDirection::Up,
                        "D" => StartDirection::Down,
                        _ => StartDirection::Right,
                    };
                }
                "FI" => {
                    if let Some((left, right)) = right.split_once(";") {
                        let size: f32 = right.trim().parse().unwrap();
                        if let Some((left, right)) = left.split_once(",") {
                            let x: f32 = left.trim().parse().unwrap();
                            let y: f32 = right.trim().parse().unwrap();
                            finish.p1.x = x;
                            finish.p1.y = y;
                            finish.p2.x = x + size;
                            finish.p2.y = y;
                            finish.p3.x = x + size;
                            finish.p3.y = y + size;
                            finish.p4.x = x + size;
                            finish.p4.y = y;

                            finish.p1 *= 50.0;
                            finish.p2 *= 50.0;
                            finish.p3 *= 50.0;
                            finish.p4 *= 50.0;
                        }
                    }
                }
                "FR" => {
                    friction = right.trim().parse().unwrap();
                }
                _ => {
                    if left.starts_with(".R") {
                        let row: f32 = left[2..].parse().unwrap();
                        for (min, max) in right.split(",").flat_map(|s| {
                            if let Some((left, right)) = s.split_once("-") {
                                Some((
                                    left.trim().parse::<u32>().unwrap(),
                                    right.trim().parse::<u32>().unwrap(),
                                ))
                            } else {
                                None
                            }
                        }) {
                            walls.push(
                                Rectangle {
                                    p1: vec2(min as f32, row) * 50.0,
                                    p2: vec2(max as f32, row) * 50.0,
                                    p3: vec2(max as f32, row) * 50.0 + vec2(0.0, 1.0),
                                    p4: vec2(min as f32, row) * 50.0 + vec2(0.0, 1.0),
                                }
                                .into(),
                            );
                        }
                    } else if left.starts_with(".C") {
                        let col: f32 = left[2..].parse().unwrap();
                        for (min, max) in right.split(",").flat_map(|s| {
                            if let Some((left, right)) = s.split_once("-") {
                                Some((
                                    left.trim().parse::<u32>().unwrap(),
                                    right.trim().parse::<u32>().unwrap(),
                                ))
                            } else {
                                None
                            }
                        }) {
                            walls.push(
                                Rectangle {
                                    p1: vec2(col, min as f32) * 50.0,
                                    p2: vec2(col, max as f32) * 50.0,
                                    p3: vec2(col, max as f32) * 50.0 + vec2(1.0, 0.0),
                                    p4: vec2(col, min as f32) * 50.0 + vec2(1.0, 0.0),
                                }
                                .into(),
                            );
                        }
                    }
                }
            }
        }
    }

    Maze {
        friction,
        start,
        walls,
        start_direction,
        finish,
    }
}

#[macroquad::main("Micromouse Simulation")]
async fn main() {
    let args = Args::parse();

    match args.command {
        Command::ExampleScript => println!("{}", include_str!("../test_data/test.rhai")),
        Command::ExampleMouse => println!("{}", include_str!("../test_data/mouse.toml")),
        Command::ExampleMaze => println!("{}", include_str!("../test_data/example.maze")),
        Command::Simulate {
            maze,
            mouse,
            script,
        } => {
            let maze = std::fs::read_to_string(maze).unwrap();
            let maze = parse_maze(&maze);

            let mouse_config: MouseConfig =
                toml::from_str(&std::fs::read_to_string(mouse).unwrap()).unwrap();

            let mut sim = Simulation::new(script, maze, mouse_config); // Create a 10x10 maze

            let mut paused = true;

            // Update the simulation
            sim.update(0.0);

            loop {
                if is_key_pressed(KeyCode::Space) {
                    paused = !paused;
                }

                let dt = get_frame_time();
                if !paused && !sim.collided {
                    let mut scope = Scope::new();
                    let mut mouse_data = sim.mouse.get_data(dt, sim.collided);
                    scope.push("mouse", mouse_data);

                    sim.engine.run_ast_with_scope(&mut scope, &sim.ast).unwrap();

                    mouse_data = scope.get_value("mouse").unwrap();
                    sim.mouse.update_from_data(mouse_data);

                    sim.update(dt);
                }

                // Render the simulation
                sim.render();

                // Control the simulation speed (Q to slow down, E to speed up)
                if is_key_pressed(KeyCode::Q) {
                    sim.time_scale = (sim.time_scale * 0.9).max(0.1);
                } else if is_key_pressed(KeyCode::E) {
                    sim.time_scale = (sim.time_scale * 1.1).min(10.0);
                }

                // Exit the simulation with ESC
                if is_key_pressed(KeyCode::Escape) {
                    break;
                }

                next_frame().await;
            }
        }
    }
}
