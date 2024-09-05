use std::{ops::Deref, str::FromStr};

pub use mazeparser::StartDirection;
use notan::math::{vec2, Vec2};

#[derive(Debug)]
pub struct Wall(Rectangle);

impl Deref for Wall {
    type Target = Rectangle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default)]
pub struct Rectangle {
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2,
    pub p4: Vec2,
}

impl From<Rectangle> for Wall {
    fn from(value: Rectangle) -> Self {
        Wall(value)
    }
}

#[derive(Debug)]
pub struct Maze {
    pub walls: Vec<Wall>, // 2D grid representing walls in each cell
    pub friction: f32,    // Friction coefficient of the maze surface
    pub start: Vec2,
    pub start_direction: StartDirection,
    pub finish: Rectangle,
}

impl Maze {
    pub fn from_string(s: &str, cell_size: f32) -> Result<Maze, String> {
        let maze = mazeparser::Maze::from_str(s)?;
        let mut walls = Vec::new();
        for wall in maze.walls {
            if let mazeparser::Orientation::Vertical = wall.orientation {
                walls.push(
                    Rectangle {
                        p1: wall.start * cell_size,
                        p2: wall.end * cell_size,
                        p3: wall.end * cell_size + vec2(0.0, -1.0),
                        p4: wall.start * cell_size + vec2(0.0, -1.0),
                    }
                    .into(),
                );
            } else {
                walls.push(
                    Rectangle {
                        p1: wall.start * cell_size,
                        p2: wall.end * cell_size,
                        p3: wall.end * cell_size + vec2(-1.0, 0.0),
                        p4: wall.start * cell_size + vec2(-1.0, 0.0),
                    }
                    .into(),
                );
            }
        }
        Ok(Maze {
            walls,
            friction: maze.friction,
            start: maze.start * cell_size,
            start_direction: maze.start_direction,
            finish: Rectangle {
                p1: maze.finish.start * cell_size,
                p2: vec2(maze.finish.start.x, maze.finish.end.y) * cell_size,
                p3: maze.finish.end * cell_size,
                p4: vec2(maze.finish.end.x, maze.finish.start.y) * cell_size,
            },
        })
    }
}
