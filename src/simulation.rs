use std::path::Path;

use macroquad::prelude::*;
use rhai::{Engine, AST};

use crate::{
    engine::build_engine,
    helper::{DOWN, LEFT, RIGHT, UP},
    maze::{Maze, StartDirection, Wall},
    mouse::{Micromouse, MouseConfig},
    ray::Ray,
};

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

pub struct Simulation {
    pub engine: Engine,
    pub mouse: Micromouse,
    pub collided: bool,
    pub finished: bool,
    pub maze: Maze,
    pub time_scale: f32, // Speed factor for the simulation and replay
    pub ast: AST,
}

impl Simulation {
    pub fn new<P: AsRef<Path>>(script: P, maze: Maze, mouse_config: MouseConfig) -> Self {
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

    pub fn update(&mut self, dt: f32) {
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

    pub fn render(&self) {
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
