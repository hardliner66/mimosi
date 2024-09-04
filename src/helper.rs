use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

pub const RIGHT: f32 = 0.0;
pub const UP: f32 = std::f32::consts::FRAC_PI_2;
pub const LEFT: f32 = std::f32::consts::PI;
pub const DOWN: f32 = 3.0 * std::f32::consts::FRAC_PI_2;

#[derive(Serialize, Deserialize)]
#[serde(remote = "Vec2")]
pub struct Vec2Def {
    pub x: f32,
    pub y: f32,
}
