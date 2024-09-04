use std::{ops::Deref, str::FromStr};

use macroquad::math::{vec2, Vec2};
use serde::{Deserialize, Serialize};

use crate::helper::Vec2Def;

#[derive(Serialize, Deserialize, Debug)]
pub struct Wall(Rectangle);

impl Deref for Wall {
    type Target = Rectangle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Rectangle {
    #[serde(with = "Vec2Def")]
    pub p1: Vec2,
    #[serde(with = "Vec2Def")]
    pub p2: Vec2,
    #[serde(with = "Vec2Def")]
    pub p3: Vec2,
    #[serde(with = "Vec2Def")]
    pub p4: Vec2,
}

impl From<Rectangle> for Wall {
    fn from(value: Rectangle) -> Self {
        Wall(value)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StartDirection {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Maze {
    pub walls: Vec<Wall>, // 2D grid representing walls in each cell
    pub friction: f32,    // Friction coefficient of the maze surface
    #[serde(with = "Vec2Def")]
    pub start: Vec2,
    pub start_direction: StartDirection,
    pub finish: Rectangle,
}

impl FromStr for Maze {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
                            start = vec2(left.trim().parse().unwrap(), right.parse().unwrap())
                                * 50.0
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
                        } else {
                            Err(format!("Invalid line: {line}"))?
                        }
                    }
                }
            }
        }

        Ok(Maze {
            friction,
            start,
            walls,
            start_direction,
            finish,
        })
    }
}
