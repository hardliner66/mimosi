use std::str::FromStr;

use glam::{vec2, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(remote = "Vec2")]
pub struct Vec2Def {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Wall {
    #[serde(with = "Vec2Def")]
    pub start: Vec2,
    #[serde(with = "Vec2Def")]
    pub end: Vec2,
    pub orientation: Orientation,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Finish {
    #[serde(with = "Vec2Def")]
    pub start: Vec2,
    #[serde(with = "Vec2Def")]
    pub end: Vec2,
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
    pub finish: Finish,
}

impl FromStr for Maze {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut friction = 1.0;
        let mut start = vec2(0.0, 0.0);
        let mut start_direction = StartDirection::Right;
        let mut walls = Vec::new();
        let mut finish = Finish::default();

        for (i, line) in s.lines().enumerate() {
            let i = i + 1;
            if line.trim().starts_with("#") {
                continue;
            }
            if let Some((left, right)) = line.split_once(":") {
                let left = left.trim().to_uppercase();
                match left.as_str() {
                    "#" => (),
                    "SP" => {
                        if let Some((left, right)) = right.split_once(",") {
                            start = vec2(
                                left.trim().parse().map_err(|e| {
                                    format!("Error in line {i}! X value of starting point is not a valid number. {e}")
                                })?,
                                right.parse().map_err(|e| {
                                    format!("Error in line {i}! Y value of starting point is not a valid number. {e}")
                                })?,
                            ) + vec2(0.5, 0.5);
                        }
                    }
                    "SD" => {
                        start_direction = match right.trim().to_uppercase().as_str() {
                            "L" => StartDirection::Left,
                            "U" => StartDirection::Up,
                            "D" => StartDirection::Down,
                            "R" => StartDirection::Right,
                            _ => Err(format!("Error in line {i}! Invalid Starting Direction"))?,
                        };
                    }
                    "FI" => {
                        if let Some((left, right)) = right.split_once(";") {
                            if let Some((left, right)) = left.split_once(",") {
                                let x: f32 = left.trim().parse().map_err(|e| format!("Error in line {i}! X value of start point of finish is not a valid number. {e}"))?;
                                let y: f32 = right.trim().parse().map_err(|e| format!("Error in line {i}! Y value of start point of finish is not a valid number. {e}"))?;
                                finish.start.x = x;
                                finish.start.y = y;
                            } else {
                                Err(format!(
                                    "Error in line {i}! Could not parse start point of finish"
                                ))?;
                            }

                            if let Some((left, right)) = right.split_once(",") {
                                let x: f32 = left.trim().parse().map_err(|e| {
                                    format!(
                                        "Error in line {i}! X value of end point of finish is not a valid number. {e}"
                                    )
                                })?;
                                let y: f32 = right.trim().parse().map_err(|e| {
                                    format!(
                                        "Error in line {i}! Y value of end point of finish is not a valid number. {e}"
                                    )
                                })?;
                                finish.end.x = x;
                                finish.end.y = y;
                            } else {
                                Err(format!(
                                    "Error in line {i}! Could not parse end point of finish"
                                ))?;
                            }
                        }
                    }
                    "FR" => {
                        friction = right.trim().parse().map_err(|e| {
                            format!("Error in line {i}! Could not parse friction: {e}")
                        })?;
                    }
                    _ => {
                        if let Some(left) = left.strip_prefix(".R") {
                            let row: f32 = left.parse().map_err(|e| {
                                format!("Error in line {i}! Not a valid row number: {e}")
                            })?;
                            for (min, max) in right.split(",").flat_map(|s| {
                                if let Some((left, right)) = s.split_once("-") {
                                    Some((
                                        left.trim().parse::<u32>().map_err(|e| format!("Error in line {i}! Starting point of the wall is not a valid number: {e}")),
                                        right.trim().parse::<u32>().map_err(|e| format!("Error in line {i}! End point of the wall is not a valid number: {e}")),
                                    ))
                                } else {
                                    None
                                }
                            }) {
                                walls.push(Wall {
                                    start: vec2(min? as f32, row),
                                    end: vec2(max? as f32, row),
                                    orientation: Orientation::Horizontal,
                                });
                            }
                        } else if let Some(left) = left.strip_prefix(".C") {
                            let col: f32 = left.parse().map_err(|e| {
                                format!("Error in line {i}! Not a valid column number: {e}")
                            })?;
                            for (min, max) in right.split(",").flat_map(|s| {
                                if let Some((left, right)) = s.split_once("-") {
                                    Some((
                                        left.trim().parse::<u32>().map_err(|e| format!("Error in line {i}! Starting point of the wall is not a valid number: {e}")),
                                        right.trim().parse::<u32>().map_err(|e| format!("Error in line {i}! End point of the wall is not a valid number: {e}")),
                                    ))
                                } else {
                                    None
                                }
                            }) {
                                walls.push(Wall {
                                    start: vec2(col, min? as f32),
                                    end: vec2(col, max? as f32),
                                    orientation: Orientation::Vertical,
                                });
                            }
                        } else {
                            Err(format!("Error in line {i}! Invalid line: {line}"))?
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
