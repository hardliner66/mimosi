use std::{
    collections::HashMap,
    fmt::Debug,
    ops::Deref,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use macroquad::prelude::*;
use rhai::{
    packages::{CorePackage, Package},
    CustomType, Dynamic, Engine, Scope, TypeBuilder, AST,
};
use serde::{Deserialize, Serialize};

const RIGHT: f32 = 0.0;
const UP: f32 = std::f32::consts::FRAC_PI_2;
const LEFT: f32 = std::f32::consts::PI;
const DOWN: f32 = 3.0 * std::f32::consts::FRAC_PI_2;

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

            if denom.abs() < f32::EPSILON {
                continue;
            }

            let t1 = ray_to_wall_start.dot(perp_wall_dir) / denom;
            let t2 = ray_to_wall_start.dot(self.direction.perp()) / denom;

            if t1 >= 0.0 && (0.0..=1.0).contains(&t2) {
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
        Sensor {
            position_offset,
            angle,
            value,
            ..
        }: &Sensor,
    ) -> Self {
        Self {
            position_offset: *position_offset,
            angle: angle.to_degrees(),
            value: *value,
        }
    }
}

impl From<Sensor> for SensorInfo {
    fn from(sensor: Sensor) -> Self {
        (&sensor).into()
    }
}

#[derive(Clone, CustomType, Debug)]
struct Sensors(#[rhai_type(skip)] HashMap<String, SensorInfo>);

impl IntoIterator for Sensors {
    type Item = (String, SensorInfo);

    type IntoIter = std::collections::hash_map::IntoIter<String, SensorInfo>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Sensors {
    fn get_sensors(&mut self, index: &str) -> SensorInfo {
        self.0[index].clone()
    }
}

#[derive(Clone, CustomType, Debug)]
struct MouseData {
    #[rhai_type(readonly)]
    wheel_base: f32,
    #[rhai_type(readonly)]
    tire_friction: f32,
    #[rhai_type(readonly)]
    mass: f32, // Mass of the micromouse

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

    #[rhai_type(readonly)]
    left_encoder: usize,
    #[rhai_type(readonly)]
    right_encoder: usize,

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
    wheel_base: f32, // Distance between the wheels
    wheel_radius: f32,
    tire_friction: f32,
    mass: f32, // Mass of the micromouse
    max_speed: f32,

    width: f32,  // Width of the mouse
    length: f32, // Length of the mouse (not including the triangle)

    encoder_resolution: usize,

    sensors: HashMap<String, Sensor>,
}

struct Micromouse {
    position: Vec2,
    width: f32,  // Width of the mouse
    length: f32, // Length of the mouse (not including the triangle)
    sensors: HashMap<String, Sensor>,

    tire_friction: f32,
    orientation: f32, // Orientation angle in radians
    wheel_base: f32,  // Distance between the wheels
    left_power: f32,
    right_power: f32,
    left_encoder: usize,
    right_encoder: usize,
    encoder_resolution: usize,

    wheel_radius: f32,
    left_velocity: f32,  // Current velocity of the left wheels
    right_velocity: f32, // Current velocity of the right wheels
    max_speed: f32,
    mass: f32, // Mass of the micromouse
}

impl Micromouse {
    fn new(
        MouseConfig {
            wheel_base,
            wheel_radius,
            width,
            length,
            sensors,
            mass,
            max_speed,
            tire_friction,
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
            tire_friction,
            left_velocity: 0.0,
            right_velocity: 0.0,
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
            left_power,
            right_power,
            left_encoder,
            right_encoder,
            mass,
            ..
        } = &self;
        MouseData {
            delta_time,
            wheel_base: *wheel_base,
            tire_friction: *tire_friction,
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
            crashed,
        }
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

    fn calculate_acceleration(&self, power: f32, current_velocity: f32, maze_friction: f32) -> f32 {
        // Force applied by the motor (simple model: power * max force)
        let motor_force = power * self.max_speed;

        // Frictional force
        let friction_force = (self.tire_friction + maze_friction) * current_velocity.abs();

        // Net force = motor force - frictional force
        let net_force = motor_force - friction_force.copysign(motor_force);

        // Acceleration = net force / mass
        net_force / self.mass
    }

    fn apply_friction(&mut self, dt: f32, maze_friction: f32) {
        // Reduce the wheel velocities due to friction
        let friction_force = self.tire_friction + maze_friction;

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

    fn update_wheel_encoders(&mut self, dt: f32) {
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

        for sensor in self.mouse.sensors.values_mut() {
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
        false
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

        for sensor in self.mouse.sensors.values() {
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
                    if let Some(left) = left.strip_prefix(".R") {
                        let row: f32 = left.parse().unwrap();
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
                    } else if let Some(left) = left.strip_prefix(".C") {
                        let col: f32 = left.parse().unwrap();
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

            let mut scope = Scope::new();
            scope.push_dynamic("state", Dynamic::from_map(Default::default()));

            loop {
                if is_key_pressed(KeyCode::Space) {
                    paused = !paused;
                }

                let dt = get_frame_time();
                if !paused && !sim.collided {
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
