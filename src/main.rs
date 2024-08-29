use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use clap::Parser;
use macroquad::prelude::*;
use rhai::{
    packages::{CorePackage, Package},
    CustomType, Engine, Scope, TypeBuilder, AST,
};

pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(128, 64);

    let package = CorePackage::new();

    // Register the package into the 'Engine' by converting it into a shared module.
    engine.register_global_module(package.as_shared_module());

    engine
        .build_type::<RaycastResult>()
        .register_fn("to_debug", |r: RaycastResult| format!("{}", r.distance))
        .register_fn("to_string", |r: RaycastResult| format!("{}", r.distance))
        .build_type::<Micromouse>()
        .build_type::<Sensors>()
        .register_indexer_get(Sensors::get_sensors);

    engine
}

#[derive(Clone, CustomType, Debug)]
struct Sensors(#[rhai_type(skip)] [RaycastResult; 8]);

impl Sensors {
    fn get_sensors(&mut self, index: i64) -> RaycastResult {
        self.0[index as usize]
    }
}

#[derive(Clone, CustomType, Debug)]
struct Micromouse {
    #[rhai_type(readonly)]
    sensors: Sensors,
    #[rhai_type(skip)]
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
}

impl Micromouse {
    fn new() -> Self {
        Self {
            sensors: Sensors([Default::default(); 8]),
            position: vec2(1.5, 1.5),
            direction: 0.0,
            left_power: 0.0,
            right_power: 0.0,
            left_velocity: 0.0,
            right_velocity: 0.0,
            max_speed: 5.0,     // Example max speed
            mass: 1.0,          // Example mass
            wheel_base: 0.5,    // Distance between wheels
            tire_friction: 0.8, // Example tire friction coefficient
            width: 0.3,         // Example width of the mouse
            length: 0.5,        // Example length of the mouse (rectangle part)
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

struct Maze {
    width: usize,
    height: usize,
    walls: Vec<Vec<CellWalls>>, // 2D grid representing walls in each cell
    friction: f32,              // Friction coefficient of the maze surface
}

#[derive(Clone, Copy, Default)]
struct CellWalls {
    north: bool,
    south: bool,
    east: bool,
    west: bool,
}

#[derive(Debug, Clone, Copy, Default, CustomType)]
struct RaycastResult {
    #[rhai_type(readonly)]
    distance: f32,
    hit_point: Vec2,
}

struct Simulation {
    engine: Engine,
    mouse: Micromouse,
    maze: Maze,
    time_scale: f32, // Speed factor for the simulation and replay
    ast: AST,
}

impl Simulation {
    fn new<P: AsRef<Path>>(maze_width: usize, maze_height: usize, script: P) -> Self {
        let engine = build_engine();
        let ast = engine.compile_file(script.as_ref().to_path_buf()).unwrap();
        Self {
            mouse: Micromouse::new(),
            maze: Maze {
                friction: 0.2, // Example maze friction coefficient
                width: maze_width,
                height: maze_height,
                walls: vec![vec![CellWalls::default(); maze_height]; maze_width],
            },
            time_scale: 1.0,
            engine,
            ast,
        }
    }

    fn update(&mut self, dt: f32) {
        let dt_scaled = dt * self.time_scale;

        self.mouse.sensors = Sensors(self.raycast_from_edges(&self.mouse));

        let mut scope = Scope::new();
        scope.push("mouse", self.mouse.clone());

        self.engine
            .run_ast_with_scope(&mut scope, &self.ast)
            .unwrap();

        self.mouse = scope.get_value("mouse").unwrap();

        self.mouse.update(dt_scaled, self.maze.friction);
        self.check_collisions();
    }

    fn check_collisions(&mut self) {
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

        // Check each corner for collision with the maze boundaries
        let corners = [rear_left, rear_right, front_left, front_right];
        for &corner in &corners {
            if corner.x < 0.0 {
                self.handle_wall_collision(corner, Vec2::new(-1.0, 0.0));
            } else if corner.x > (self.maze.width as f32) {
                self.handle_wall_collision(corner, Vec2::new(1.0, 0.0));
            }

            if corner.y < 0.0 {
                self.handle_wall_collision(corner, Vec2::new(0.0, -1.0));
            } else if corner.y > (self.maze.height as f32) {
                self.handle_wall_collision(corner, Vec2::new(0.0, 1.0));
            }
        }

        // Check internal walls
        for &corner in &corners {
            let cell_x = corner.x.floor() as usize;
            let cell_y = corner.y.floor() as usize;

            if cell_x < self.maze.width && cell_y < self.maze.height {
                let walls = self.maze.walls[cell_x][cell_y];

                if walls.north && corner.y.fract() < 0.1 {
                    self.handle_wall_collision(corner, Vec2::new(0.0, -1.0));
                    return;
                }
                if walls.south && corner.y.fract() > 0.9 {
                    self.handle_wall_collision(corner, Vec2::new(0.0, 1.0));
                    return;
                }
                if walls.west && corner.x.fract() < 0.1 {
                    self.handle_wall_collision(corner, Vec2::new(-1.0, 0.0));
                    return;
                }
                if walls.east && corner.x.fract() > 0.9 {
                    self.handle_wall_collision(corner, Vec2::new(1.0, 0.0));
                    return;
                }
            }
        }
    }

    fn handle_wall_collision(&mut self, corner: Vec2, normal: Vec2) {
        let mouse = &mut self.mouse;
        let restitution = 0.3; // Coefficient of restitution (bounciness)

        // Calculate the penetration depth
        let penetration_depth = (corner - mouse.position).dot(normal);

        // Adjust the mouse position based on the penetration depth
        mouse.position -= normal * penetration_depth;

        // Reflect the velocity component normal to the wall
        let velocity_normal = (mouse.left_velocity * mouse.direction.cos()
            + mouse.right_velocity * mouse.direction.sin())
            * normal;
        mouse.left_velocity -= 2.0 * velocity_normal.x * normal.x * restitution;
        mouse.right_velocity -= 2.0 * velocity_normal.y * normal.y * restitution;
    }

    fn perform_raycast(&self, origin: Vec2, direction: Vec2) -> RaycastResult {
        let mut closest_distance = f32::MAX;
        let mut closest_point = origin + direction * closest_distance;

        // Check for intersection with the maze boundaries
        let maze_width = self.maze.width as f32;
        let maze_height = self.maze.height as f32;

        // Check intersections with the maze boundaries
        if direction.x != 0.0 {
            if direction.x > 0.0 {
                let t = (maze_width - origin.x) / direction.x;
                if t >= 0.0 {
                    let y_intersection = origin.y + t * direction.y;
                    if y_intersection >= 0.0 && y_intersection <= maze_height {
                        closest_distance = t;
                        closest_point = origin + direction * t;
                    }
                }
            } else {
                let t = (0.0 - origin.x) / direction.x;
                if t >= 0.0 {
                    let y_intersection = origin.y + t * direction.y;
                    if y_intersection >= 0.0 && y_intersection <= maze_height {
                        closest_distance = t;
                        closest_point = origin + direction * t;
                    }
                }
            }
        }

        if direction.y != 0.0 {
            if direction.y > 0.0 {
                let t = (maze_height - origin.y) / direction.y;
                if t >= 0.0 {
                    let x_intersection = origin.x + t * direction.x;
                    if x_intersection >= 0.0 && x_intersection <= maze_width {
                        if t < closest_distance {
                            closest_distance = t;
                            closest_point = origin + direction * t;
                        }
                    }
                }
            } else {
                let t = (0.0 - origin.y) / direction.y;
                if t >= 0.0 {
                    let x_intersection = origin.x + t * direction.x;
                    if x_intersection >= 0.0 && x_intersection <= maze_width {
                        if t < closest_distance {
                            closest_distance = t;
                            closest_point = origin + direction * t;
                        }
                    }
                }
            }
        }

        // Check intersections with internal walls
        for x in 0..self.maze.width {
            for y in 0..self.maze.height {
                let walls = self.maze.walls[x][y];
                let cell_x = x as f32;
                let cell_y = y as f32;

                // Check north wall
                if walls.north {
                    let t = (cell_y - origin.y) / direction.y;
                    if t >= 0.0 {
                        let x_intersection = origin.x + t * direction.x;
                        if x_intersection >= cell_x && x_intersection <= cell_x + 1.0 {
                            if t < closest_distance {
                                closest_distance = t;
                                closest_point = origin + direction * t;
                            }
                        }
                    }
                }
                // Check south wall
                if walls.south {
                    let t = (cell_y + 1.0 - origin.y) / direction.y;
                    if t >= 0.0 {
                        let x_intersection = origin.x + t * direction.x;
                        if x_intersection >= cell_x && x_intersection <= cell_x + 1.0 {
                            if t < closest_distance {
                                closest_distance = t;
                                closest_point = origin + direction * t;
                            }
                        }
                    }
                }
                // Check west wall
                if walls.west {
                    let t = (cell_x - origin.x) / direction.x;
                    if t >= 0.0 {
                        let y_intersection = origin.y + t * direction.y;
                        if y_intersection >= cell_y && y_intersection <= cell_y + 1.0 {
                            if t < closest_distance {
                                closest_distance = t;
                                closest_point = origin + direction * t;
                            }
                        }
                    }
                }
                // Check east wall
                if walls.east {
                    let t = (cell_x + 1.0 - origin.x) / direction.x;
                    if t >= 0.0 {
                        let y_intersection = origin.y + t * direction.y;
                        if y_intersection >= cell_y && y_intersection <= cell_y + 1.0 {
                            if t < closest_distance {
                                closest_distance = t;
                                closest_point = origin + direction * t;
                            }
                        }
                    }
                }
            }
        }

        RaycastResult {
            distance: closest_distance,
            hit_point: closest_point,
        }
    }

    fn raycast_from_edges(&self, mouse: &Micromouse) -> [RaycastResult; 8] {
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

        // Directions to cast rays
        let directions = [
            vec2(1.0, 0.0).rotate(Vec2::from_angle(mouse.direction)), // Front
            vec2(-1.0, 0.0).rotate(Vec2::from_angle(mouse.direction)), // Back
            vec2(0.0, 1.0).rotate(Vec2::from_angle(mouse.direction)), // Right
            vec2(0.0, -1.0).rotate(Vec2::from_angle(mouse.direction)), // Left
        ];

        // Perform raycasts from each corner in the specified directions
        [
            self.perform_raycast(front_left, directions[0]),
            self.perform_raycast(front_right, directions[0]),
            self.perform_raycast(rear_left, directions[1]),
            self.perform_raycast(rear_right, directions[1]),
            self.perform_raycast(front_left, directions[3]),
            self.perform_raycast(rear_left, directions[3]),
            self.perform_raycast(front_right, directions[2]),
            self.perform_raycast(rear_right, directions[2]),
        ]
    }

    fn render(&self) {
        clear_background(LIGHTGRAY);

        // Render the maze with internal and outside walls
        self.render_maze();

        // Render the mouse
        self.render_mouse();
    }

    fn render_maze(&self) {
        for x in 0..self.maze.width {
            for y in 0..self.maze.height {
                let walls = self.maze.walls[x][y];

                let cell_x = x as f32 * 50.0;
                let cell_y = y as f32 * 50.0;

                // Draw the cell boundaries (internal walls)
                if walls.north {
                    draw_line(cell_x, cell_y, cell_x + 50.0, cell_y, 2.0, BLACK);
                }
                if walls.south {
                    draw_line(
                        cell_x,
                        cell_y + 50.0,
                        cell_x + 50.0,
                        cell_y + 50.0,
                        2.0,
                        BLACK,
                    );
                }
                if walls.west {
                    draw_line(cell_x, cell_y, cell_x, cell_y + 50.0, 2.0, BLACK);
                }
                if walls.east {
                    draw_line(
                        cell_x + 50.0,
                        cell_y,
                        cell_x + 50.0,
                        cell_y + 50.0,
                        2.0,
                        BLACK,
                    );
                }
            }
        }

        // Draw the outside walls of the maze
        let width = self.maze.width as f32 * 50.0;
        let height = self.maze.height as f32 * 50.0;

        // Top wall
        draw_line(0.0, 0.0, width, 0.0, 2.0, BLACK);
        // Bottom wall
        draw_line(0.0, height, width, height, 2.0, BLACK);
        // Left wall
        draw_line(0.0, 0.0, 0.0, height, 2.0, BLACK);
        // Right wall
        draw_line(width, 0.0, width, height, 2.0, BLACK);
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
        draw_triangle(rear_left * 50.0, rear_right * 50.0, front_right * 50.0, RED);
        draw_triangle(rear_left * 50.0, front_left * 50.0, front_right * 50.0, RED);

        // Draw the triangular front
        draw_triangle(
            front_left * 50.0,
            front_right * 50.0,
            front_center * 50.0,
            BLUE,
        );

        // Draw the rays
        for result in mouse.sensors.0.iter() {
            draw_line(
                mouse.position.x * 50.0,
                mouse.position.y * 50.0,
                result.hit_point.x * 50.0,
                result.hit_point.y * 50.0,
                1.0,
                DARKPURPLE,
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

    let mut sim = Simulation::new(10, 10, args.path); // Create a 10x10 maze

    // Set up some internal walls
    sim.maze.walls[2][2].east = true;
    sim.maze.walls[3][2].west = true;
    sim.maze.walls[4][4].north = true;
    sim.maze.walls[4][3].south = true;
    sim.maze.walls[5][5].west = true;
    sim.maze.walls[5][5].south = true;

    let mut paused = true;
    loop {
        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
        }

        if !paused {
            let dt = get_frame_time();

            // Update the simulation
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
