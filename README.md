# MiMoSi - The MIcro MOuse SImulator

MiMoSi is supposed to be a semi-realistic micromouse simulator.

The mouse has independently controllable wheels (left/right side) and a few basic sensors.

Currently it can be controlled with rhai scripts,
but I'm planning on adding wasm support through [extism](https://extism.org/) later on.
This should enable people to write their code in every language thats supported by extism.

Other script engines might be added later on.
For now its important that the scripts can be run without posing a risk to the person running the scripts.

## Features
- A custom text format to define mazes
- A configurable mouse
  - Width
  - Length
  - Mass
  - Max Speed
  - Wheel Base
  - Wheel Radius
  - Wheel Encoder Resolution (ticks per full wheel turn)
  - Wheel Friction
  - Sensors
    - Offset from Mouse
    - Angle

## Try it yourself
```sh
git clone https://github.com/hardliner66/mimosi
cd mimosi
cargo run -- simulate test_data/example.maze test_data/mouse.toml test_data/test.rhai
```

## Rhai API

The mouse is controlled through a single variable called `mouse`.
```rs
mouse: MouseData;
```

The variable has the following type:
```rs
struct MouseData {
    // The delta time since the last call
    #[read_only]
    delta_time: f32,

    // Width of the mouse
    #[read_only]
    width : f32,

    // Length of the mouse (not including the triangle)
    #[read_only]
    length: f32,

    // Mass of the micromouse
    #[read_only]
    mass  : f32, 

    // The distance between the wheels
    #[read_only]
    wheel_base        : f32,

    // The friction of the wheels
    #[read_only]
    wheel_friction    : f32,

    // How many ticks the encoder counts per turn
    #[read_only]
    encoder_resolution: usize,

    // if the mouse has crashed
    #[read_only]
    crashed: bool,

    // a dictionary of sensors
    #[read_only]
    sensors: HashMap<String, SensorInfo>,

    // How many ticks the left encoder measured
    #[read_only]
    left_encoder: usize,

    // How many ticks the right encoder measured
    #[read_only]
    right_encoder: usize,

    // How much power to set the left wheels to. (-1..=1)
    left_power: f32,

    // How much power to set the right wheels to. (-1..=1)
    right_power: f32,
}

struct SensorInfo {
    // The offset, relative to the center of the rectangle, at which the sensor is mounted
    #[readonly]
    position_offset: Vec2,

    // The angle at which the sensor is mounted
    #[readonly]
    angle: f32,

    // the distance to the next wall detected by the sensor
    #[readonly]
    value: f32,
}
```

Check out [test_data/test.rhai](./test_data/test.rhai) for an example on how to use the API.
Check out the [Rhai Book](https://rhai.rs/book/) to learn more about rhai.

## Maze Text Format
| Key                     | Description                                                                                   |
| ----------------------- | --------------------------------------------------------------------------------------------- |
| SP                      | Starting Point. Which cell the mouse starts in. Format: x, y                                  |
| SD                      | Starting Direction. Which direction the mouse should face to start. Can be one of: R, L, U, D |
| FI                      | Finish. Where the finish should be placed. Format: x,y; size                                  |
| FR                      | Maze Friction.                                                                                |
| .R followed by a number | Defines walls in the row with the number after .R                                             |
| .C followed by a number | Defines walls in the column with the number after .C                                          |

Lines without `:` and lines starting with a `#` are ignored.

For an example see: [test_data/example.maze](./test_data/example.maze)

## Planned features
- WASM plugins
- UI for running locally and on the web
- Replay system to help with debugging
- More realistic physics (maybe even 3d)
- Scoring system (maybe)
