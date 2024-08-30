use std::{
    fmt::Debug,
    ops::Deref,
    path::{Path, PathBuf},
};

use clap::Parser;
use macroquad::prelude::*;
use rhai::{
    packages::{CorePackage, Package},
    CustomType, Engine, Scope, TypeBuilder, AST,
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
        .build_type::<Micromouse>()
        .build_type::<Sensor>()
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

#[derive(Clone, CustomType, Debug, Default, Serialize, Deserialize)]
struct Sensor {
    #[rhai_type(readonly)]
    #[serde(with = "Vec2Def")]
    position_offset: Vec2, // Offset relative to the center of the rectangle
    #[rhai_type(readonly)]
    angle: f32, // Angle in radians
    #[rhai_type(readonly)]
    value: f32,
    #[rhai_type(skip)]
    #[serde(skip)]
    closest_point: Vec2,
}

#[derive(Clone, CustomType, Debug, Serialize, Deserialize)]
struct Sensors(#[rhai_type(skip)] [Sensor; 8]);

impl Sensors {
    fn get_sensors(&mut self, index: i64) -> Sensor {
        self.0[index as usize].clone()
    }
}

#[derive(Clone, CustomType, Debug, Serialize, Deserialize)]
struct Micromouse {
    #[rhai_type(skip)]
    #[serde(with = "Vec2Def")]
    position: Vec2,
    #[rhai_type(skip)]
    direction: f32, // Current direction in radians
    #[rhai_type(set = Micromouse::set_left_power)]
    left_power: f32, // Power input to the left wheels (0 to 1)
    #[rhai_type(set = Micromouse::set_right_power)]
    right_power: f32, // Power input to the right wheels (0 to 1)
    #[rhai_type(readonly)]
    left_velocity: f32, // Current velocity of the left wheels
    #[rhai_type(readonly)]
    right_velocity: f32, // Current velocity of the right wheels
    #[rhai_type(readonly)]
    max_speed: f32, // Maximum speed of the mouse (units per second)
    #[rhai_type(readonly)]
    mass: f32, // Mass of the mouse
    #[rhai_type(readonly)]
    wheel_base: f32, // Distance between the two wheels
    #[rhai_type(readonly)]
    tire_friction: f32, // Friction coefficient of the tires
    #[rhai_type(readonly)]
    width: f32, // Width of the mouse
    #[rhai_type(readonly)]
    length: f32, // Length of the mouse (not including the triangle)
    #[rhai_type(readonly)]
    sensors: Sensors,
}

impl Micromouse {
    fn new(position: Vec2, direction: f32) -> Self {
        let width = 15.0;
        let length = 25.0;

        let half_width = width / 2.0;
        let half_length = length / 2.0;

        Self {
            position,
            direction,
            left_power: 0.0,
            right_power: 0.0,
            left_velocity: 0.0,
            right_velocity: 0.0,
            max_speed: 2000.0,
            mass: 1.0,
            wheel_base: 25.0,
            tire_friction: 0.8,
            width,
            length,
            sensors: Sensors([
                Sensor {
                    position_offset: Vec2 {
                        x: half_length,
                        y: half_width,
                    },
                    angle: RIGHT,
                    ..Default::default()
                },
                Sensor {
                    position_offset: Vec2 {
                        x: half_length,
                        y: half_width,
                    },
                    angle: DOWN,
                    ..Default::default()
                },
                Sensor {
                    position_offset: Vec2 {
                        x: half_length,
                        y: -half_width,
                    },
                    angle: RIGHT,
                    ..Default::default()
                },
                Sensor {
                    position_offset: Vec2 {
                        x: half_length,
                        y: -half_width,
                    },
                    angle: UP,
                    ..Default::default()
                },
                Sensor {
                    position_offset: Vec2 {
                        x: -half_length,
                        y: -half_width,
                    },
                    angle: UP,
                    ..Default::default()
                },
                Sensor {
                    position_offset: Vec2 {
                        x: -half_length,
                        y: -half_width,
                    },
                    angle: LEFT,
                    ..Default::default()
                },
                Sensor {
                    position_offset: Vec2 {
                        x: -half_length,
                        y: half_width,
                    },
                    angle: LEFT,
                    ..Default::default()
                },
                Sensor {
                    position_offset: Vec2 {
                        x: -half_length,
                        y: half_width,
                    },
                    angle: DOWN,
                    ..Default::default()
                },
            ]),
        }
    }

    fn set_left_power(&mut self, power: f32) {
        self.left_power = power.clamp(-1.0, 1.0);
    }

    fn set_right_power(&mut self, power: f32) {
        self.right_power = power.clamp(-1.0, 1.0);
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
        let turning_rate = (self.right_velocity - self.left_velocity) / self.wheel_base;

        // Update direction and position
        self.direction += turning_rate * dt;
        self.position.x += average_velocity * self.direction.cos() * dt;
        self.position.y += average_velocity * self.direction.sin() * dt;

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
}

struct MazeGenerator {
    width: usize,
    height: usize,
    cell_size: f32,
    cells: Vec<Vec<CellWalls>>, // 2D grid representing walls in each cell
    friction: f32,              // Friction coefficient of the maze surface
}

#[derive(Clone, Copy, Default)]
struct CellWalls {
    north: bool,
    south: bool,
    east: bool,
    west: bool,
}

#[derive(Serialize, Deserialize)]
enum StartDirection {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Default, Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct Maze {
    walls: Vec<Wall>, // 2D grid representing walls in each cell
    friction: f32,    // Friction coefficient of the maze surface
    #[serde(with = "Vec2Def")]
    start: Vec2,
    start_direction: StartDirection,
    finish: Rectangle,
}

fn cell_walls_to_walls(cell_position: Vec2, cell_walls: CellWalls, cell_size: f32) -> Vec<Wall> {
    let mut walls = Vec::new();

    if cell_walls.north {
        walls.push(
            Rectangle {
                p1: Vec2 {
                    x: cell_position.x,
                    y: cell_position.y + cell_size,
                },
                p2: Vec2 {
                    x: cell_position.x + cell_size,
                    y: cell_position.y + cell_size,
                },
                p3: Vec2 {
                    x: cell_position.x + cell_size,
                    y: cell_position.y + cell_size + 2.0,
                },
                p4: Vec2 {
                    x: cell_position.x,
                    y: cell_position.y + cell_size + 2.0,
                },
            }
            .into(),
        );
    }

    if cell_walls.south {
        walls.push(
            Rectangle {
                p1: Vec2 {
                    x: cell_position.x,
                    y: cell_position.y,
                },
                p2: Vec2 {
                    x: cell_position.x + cell_size,
                    y: cell_position.y,
                },
                p3: Vec2 {
                    x: cell_position.x + cell_size,
                    y: cell_position.y + 2.0,
                },
                p4: Vec2 {
                    x: cell_position.x,
                    y: cell_position.y + 2.0,
                },
            }
            .into(),
        );
    }

    if cell_walls.east {
        walls.push(
            Rectangle {
                p1: Vec2 {
                    x: cell_position.x + cell_size,
                    y: cell_position.y,
                },
                p2: Vec2 {
                    x: cell_position.x + cell_size,
                    y: cell_position.y + cell_size,
                },
                p3: Vec2 {
                    x: cell_position.x + cell_size + 2.0,
                    y: cell_position.y + cell_size,
                },
                p4: Vec2 {
                    x: cell_position.x + cell_size + 2.0,
                    y: cell_position.y,
                },
            }
            .into(),
        );
    }

    if cell_walls.west {
        walls.push(
            Rectangle {
                p1: Vec2 {
                    x: cell_position.x,
                    y: cell_position.y,
                },
                p2: Vec2 {
                    x: cell_position.x,
                    y: cell_position.y + cell_size,
                },
                p3: Vec2 {
                    x: cell_position.x + 2.0,
                    y: cell_position.y + cell_size,
                },
                p4: Vec2 {
                    x: cell_position.x + 2.0,
                    y: cell_position.y,
                },
            }
            .into(),
        );
    }

    walls
}

impl From<MazeGenerator> for Maze {
    fn from(
        MazeGenerator {
            width,
            height,
            cell_size,
            cells,
            friction,
        }: MazeGenerator,
    ) -> Self {
        let mut walls = Vec::new();
        walls.push(
            Rectangle {
                p1: vec2(0.0, 0.0),
                p2: vec2(width as f32 * cell_size, 0.0),
                p3: vec2(width as f32 * cell_size, 0.0),
                p4: vec2(0.0, 0.0),
            }
            .into(),
        );

        walls.push(
            Rectangle {
                p1: vec2(0.0, 0.0),
                p2: vec2(0.0, height as f32 * cell_size),
                p3: vec2(0.0, height as f32 * cell_size),
                p4: vec2(0.0, 0.0),
            }
            .into(),
        );

        walls.push(
            Rectangle {
                p1: vec2(width as f32 * cell_size, height as f32 * cell_size),
                p2: vec2(width as f32 * cell_size, 0.0),
                p3: vec2(width as f32 * cell_size, 0.0),
                p4: vec2(width as f32 * cell_size, height as f32 * cell_size),
            }
            .into(),
        );

        walls.push(
            Rectangle {
                p1: vec2(width as f32 * cell_size, height as f32 * cell_size),
                p2: vec2(0.0, height as f32 * cell_size),
                p3: vec2(0.0, height as f32 * cell_size),
                p4: vec2(width as f32 * cell_size, height as f32 * cell_size),
            }
            .into(),
        );

        for (i, row) in cells.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                walls.append(&mut cell_walls_to_walls(
                    vec2(i as f32 * 50.0, j as f32 * 50.0),
                    *cell,
                    50.0,
                ));
            }
        }

        Self {
            walls,
            friction,
            finish: Rectangle::default(),
            start: Vec2::default(),
            start_direction: StartDirection::Right,
        }
    }
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
    maze: Maze,
    time_scale: f32, // Speed factor for the simulation and replay
    ast: AST,
}

impl Simulation {
    fn new<P: AsRef<Path>>(script: P, maze: Maze) -> Self {
        let engine = build_engine();
        let ast = engine.compile_file(script.as_ref().to_path_buf()).unwrap();
        Self {
            mouse: Micromouse::new(
                maze.start,
                match maze.start_direction {
                    StartDirection::Up => UP,
                    StartDirection::Right => RIGHT,
                    StartDirection::Down => DOWN,
                    StartDirection::Left => LEFT,
                },
            ),
            collided: false,
            maze,
            time_scale: 1.0,
            engine,
            ast,
        }
    }

    fn update(&mut self, dt: f32) {
        let dt_scaled = dt * self.time_scale;

        self.mouse.update(dt_scaled, self.maze.friction);

        for sensor in &mut self.mouse.sensors.0 {
            let p = self.mouse.position
                + sensor
                    .position_offset
                    .rotate(Vec2::from_angle(self.mouse.direction));
            let angle = self.mouse.direction + sensor.angle;
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
    }

    fn check_collisions(&self) -> bool {
        let mouse = &self.mouse;

        let half_width = mouse.width / 2.0;
        let half_length = mouse.length / 2.0;

        // Calculate the corners of the rectangle
        let rear_left = mouse.position
            + vec2(-half_length, -half_width).rotate(Vec2::from_angle(mouse.direction));
        let rear_right = mouse.position
            + vec2(-half_length, half_width).rotate(Vec2::from_angle(mouse.direction));
        let front_left = mouse.position
            + vec2(half_length, -half_width).rotate(Vec2::from_angle(mouse.direction));
        let front_right = mouse.position
            + vec2(half_length, half_width).rotate(Vec2::from_angle(mouse.direction));
        let front_center = mouse.position
            + vec2(half_length + half_width, 0.0).rotate(Vec2::from_angle(mouse.direction));

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
            draw_line(wall.p1.x, wall.p1.y, wall.p2.x, wall.p2.y, 1.0, BLACK);
            draw_line(wall.p2.x, wall.p2.y, wall.p3.x, wall.p3.y, 1.0, BLACK);
            draw_line(wall.p3.x, wall.p3.y, wall.p4.x, wall.p4.y, 1.0, BLACK);
            draw_line(wall.p4.x, wall.p4.y, wall.p1.x, wall.p1.y, 1.0, BLACK);
        }
    }

    fn render_mouse(&self) {
        let mouse = &self.mouse;
        let half_width = mouse.width / 2.0;
        let half_length = mouse.length / 2.0;

        // Calculate the corners of the rectangle
        let rear_left = mouse.position
            + vec2(-half_length, -half_width).rotate(Vec2::from_angle(mouse.direction));
        let rear_right = mouse.position
            + vec2(-half_length, half_width).rotate(Vec2::from_angle(mouse.direction));
        let front_left = mouse.position
            + vec2(half_length, -half_width).rotate(Vec2::from_angle(mouse.direction));
        let front_right = mouse.position
            + vec2(half_length, half_width).rotate(Vec2::from_angle(mouse.direction));
        let front_center = mouse.position
            + vec2(half_length + half_width, 0.0).rotate(Vec2::from_angle(mouse.direction));

        // Draw the rectangle part of the mouse
        draw_triangle(rear_left, rear_right, front_right, RED);
        draw_triangle(rear_left, front_left, front_right, RED);

        // Draw the triangular front
        draw_triangle(front_left, front_right, front_center, BLUE);

        for sensor in &self.mouse.sensors.0 {
            let p1 = self.mouse.position
                + sensor
                    .position_offset
                    .rotate(Vec2::from_angle(mouse.direction));
            let p2 = sensor.closest_point;
            draw_line(p1.x, p1.y, p2.x, p2.y, 2.0, DARKPURPLE);
        }

        if self.collided {
            draw_line(
                rear_left.x,
                rear_left.y,
                front_right.x,
                front_right.y,
                2.0,
                BLACK,
            );
            draw_line(
                rear_right.x,
                rear_right.y,
                front_left.x,
                front_left.y,
                2.0,
                BLACK,
            );
        }
    }
}

#[derive(Parser)]
struct Args {
    path: PathBuf,
}

#[macroquad::main("Micromouse Simulation")]
async fn main() {
    let args = Args::parse();

    // Set up some internal walls
    let mut mg = MazeGenerator {
        height: 10,
        width: 10,
        cell_size: 50.0,
        friction: 0.8,
        cells: vec![vec![CellWalls::default(); 10]; 10],
    };
    mg.cells[2][2].east = true;
    mg.cells[3][2].west = true;
    mg.cells[4][4].north = true;
    mg.cells[4][3].south = true;
    mg.cells[5][5].west = true;
    mg.cells[5][5].south = true;

    let mut maze: Maze = mg.into();
    maze.start = vec2(75.0, 75.0);
    maze.start_direction = StartDirection::Right;

    let mut sim = Simulation::new(args.path, maze); // Create a 10x10 maze

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
            scope.push("mouse", sim.mouse.clone());

            sim.engine.run_ast_with_scope(&mut scope, &sim.ast).unwrap();

            sim.mouse = scope.get_value("mouse").unwrap();

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
