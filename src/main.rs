use clap::Parser;
use egui::{ScrollArea, Ui};
use maze::Maze;
use mouse::{Micromouse, MouseConfig};

use notan::draw::*;
use notan::egui::{self, *};
use notan::prelude::*;

use std::{fmt::Display, path::PathBuf, str::FromStr};

use args::{Args, Command};
use rhai::{Dynamic, Scope};
use simulation::Simulation;
use stringlit::s;

mod args;
mod engine;
mod helper;
mod maze;
mod mouse;
mod ray;
mod simulation;

const DEFAULT_MAZE: &str = include_str!("../test_data/example.maze");
const DEFAULT_MOUSE: &str = include_str!("../test_data/mouse.toml");
const DEFAULT_SCRIPT: &str = include_str!("../test_data/test.rhai");

fn read_with_defaults(
    maze: Option<PathBuf>,
    mouse: Option<PathBuf>,
    script: Option<PathBuf>,
) -> anyhow::Result<(String, String, String)> {
    Ok((
        maze.map(std::fs::read_to_string)
            .unwrap_or_else(|| Ok(s!(DEFAULT_MAZE)))?,
        mouse
            .map(std::fs::read_to_string)
            .unwrap_or_else(|| Ok(s!(DEFAULT_MOUSE)))?,
        script
            .map(std::fs::read_to_string)
            .unwrap_or_else(|| Ok(s!(DEFAULT_SCRIPT)))?,
    ))
}

fn value<D: Display>(ui: &mut Ui, text: &str, value: D) {
    ui.horizontal(|ui| {
        ui.label(format!("{text}:"));
        ui.label(format!("{value}"));
    });
}

fn draw(_app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
    let mut draw = gfx.create_draw();

    // Render the simulation
    state.sim.render(&mut draw);

    gfx.render(&draw);

    let output = plugins.egui(|ctx| {
        egui::SidePanel::new(egui::panel::Side::Right, "Control").show(ctx, |ui| {
            ui.checkbox(&mut state.paused, "Pause (Space)");
            ui.separator();
            ui.heading("Debug");
            value(ui, "- FPS", format!("{:.0}", state.fps));
            value(ui, "- DT", state.delta_time);

            ui.separator();
            ui.collapsing("Maze Config", |ui| {
                value(ui, "- Maze Friction", state.sim.maze.friction);
            });

            ui.separator();
            ui.collapsing("Mouse Config", |ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    value(ui, "- Crashed", state.sim.collided);
                    value(ui, "- Width", state.sim.mouse.width);
                    value(ui, "- Length", state.sim.mouse.length);
                    value(ui, "- Wheel Radius", state.sim.mouse.wheel_radius);
                    value(ui, "- Wheel Base", state.sim.mouse.wheel_base);
                    value(ui, "- Wheel Friction", state.sim.mouse.wheel_friction);
                    value(ui, "- Left Power", state.sim.mouse.left_power);
                    value(ui, "- Right Power", state.sim.mouse.right_power);
                    value(ui, "- Left Encoder", state.sim.mouse.left_encoder);
                    value(ui, "- Right Encoder", state.sim.mouse.right_encoder);

                    ui.label("Sensors:");
                    ui.label(toml::to_string_pretty(&state.sim.mouse.sensors).unwrap());
                });
            });
        });
        ctx.input(|i| {
            for f in &i.raw.dropped_files {
                if let Some(bytes) = &f.bytes {
                    let s = String::from_utf8_lossy(bytes).to_string();
                    if let Ok(config) = toml::from_str::<MouseConfig>(&s) {
                        state.sim.mouse = Micromouse::new(
                            config,
                            state.sim.mouse.position,
                            state.sim.mouse.orientation,
                        );
                    } else if let Ok(ast) = state.sim.engine.compile(&s) {
                        state.sim.ast = ast;
                    } else if let Ok(maze) = Maze::from_str(&s) {
                        state.sim.maze = maze;
                    }
                }
            }
        });
    });

    gfx.render(&output);
}

fn update(app: &mut App, state: &mut State) {
    state.delta_time = app.timer.delta_f32();
    if state.tick % 100 == 0 {
        state.fps = app.timer.fps();
    }
    if app.keyboard.is_down(KeyCode::Space) && state.pause_timer == 0 {
        state.pause_timer = 20;
        state.paused = !state.paused;
    }

    if !state.paused && !state.sim.collided {
        let mut mouse_data = state
            .sim
            .mouse
            .get_data(state.delta_time, state.sim.collided);
        state.scope.push("mouse", mouse_data);

        state
            .sim
            .engine
            .run_ast_with_scope(&mut state.scope, &state.sim.ast)
            .unwrap();

        mouse_data = state.scope.get_value("mouse").unwrap();
        state.sim.mouse.update_from_data(mouse_data);

        state.sim.update(state.delta_time);
    }

    // Exit the simulation with ESC
    #[cfg(not(target_arch = "wasm32"))]
    if app.keyboard.is_down(KeyCode::Escape) {
        std::process::exit(0);
    }

    state.tick = state.tick.wrapping_add(1);
    state.pause_timer = state.pause_timer.saturating_sub(1);
}

#[derive(AppState)]
struct State<'a> {
    sim: Simulation,
    paused: bool,
    pause_timer: usize,
    scope: Scope<'a>,
    delta_time: f32,
    tick: usize,
    fps: f32,
}

#[notan_main]
fn main() -> Result<(), String> {
    let args = Args::parse();

    match args.command.unwrap_or(Command::Simulate {
        maze: None,
        mouse: None,
        script: None,
    }) {
        Command::ExampleScript => Ok(println!("{}", DEFAULT_SCRIPT)),
        Command::ExampleMouse => Ok(println!("{}", DEFAULT_MOUSE)),
        Command::ExampleMaze => Ok(println!("{}", DEFAULT_MAZE)),
        Command::Simulate {
            maze,
            mouse,
            script,
        } => {
            let (maze, mouse, script) =
                read_with_defaults(maze, mouse, script).map_err(|e| format!("{e}"))?;
            let maze: Maze = maze.parse()?;

            let mouse_config: MouseConfig = toml::from_str(&mouse).unwrap();

            let mut sim = Simulation::new(script, maze, mouse_config);

            // Update the simulation
            sim.update(0.0);

            let win_config = WindowConfig::new().set_size(1015, 810).set_vsync(true);

            notan::init_with(|| {
                let mut scope = Scope::new();
                scope.push_dynamic("state", Dynamic::from_map(Default::default()));
                State {
                    sim,
                    paused: true,
                    pause_timer: 0,
                    scope,
                    delta_time: 0.0,
                    fps: 0.0,
                    tick: 0,
                }
            })
            .add_config(win_config)
            .add_config(DrawConfig)
            .add_config(EguiConfig)
            .update(update)
            .draw(draw)
            .build()
        }
    }
}
