use std::collections::HashMap;

use notan::math::Vec2;
use rhai::{
    packages::{CorePackage, Package},
    CustomType, Engine, TypeBuilder,
};

use crate::mouse::Sensor;

#[derive(Clone, CustomType, Debug)]
pub struct MouseData {
    #[rhai_type(readonly)]
    pub wheel_base: f32,
    #[rhai_type(readonly)]
    pub wheel_friction: f32,
    #[rhai_type(readonly)]
    pub mass: f32, // Mass of the micromouse

    pub encoder_resolution: usize,

    #[rhai_type(readonly)]
    pub crashed: bool,

    #[rhai_type(readonly)]
    pub delta_time: f32,

    #[rhai_type(readonly)]
    pub width: f32, // Width of the mouse
    #[rhai_type(readonly)]
    pub length: f32, // Length of the mouse (not including the triangle)
    #[rhai_type(readonly)]
    pub sensors: Sensors,

    #[rhai_type(readonly)]
    pub left_encoder: usize,
    #[rhai_type(readonly)]
    pub right_encoder: usize,

    #[rhai_type(set=MouseData::set_left_power, get=MouseData::get_left_power)]
    pub left_power: f32,

    #[rhai_type(set=MouseData::set_right_power, get=MouseData::get_right_power)]
    pub right_power: f32,
}

impl MouseData {
    pub fn set_left_power(&mut self, power: f32) {
        self.left_power = power.clamp(-1.0, 1.0);
    }

    pub fn get_left_power(&self) -> f32 {
        self.left_power
    }

    pub fn set_right_power(&mut self, power: f32) {
        self.right_power = power.clamp(-1.0, 1.0);
    }

    pub fn get_right_power(&self) -> f32 {
        self.right_power
    }
}

#[derive(Clone, CustomType, Debug, Default)]
pub struct SensorInfo {
    #[rhai_type(readonly)]
    pub position_offset: Vec2, // Offset relative to the center of the rectangle
    #[rhai_type(readonly)]
    pub angle: f32, // Angle in radians
    #[rhai_type(readonly)]
    pub value: f32,
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
pub struct Sensors(#[rhai_type(skip)] pub HashMap<String, SensorInfo>);

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
