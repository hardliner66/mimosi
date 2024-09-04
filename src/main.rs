use clap::Parser;
use macroquad::prelude::*;
use maze::Maze;
use mouse::MouseConfig;
use rhai::{Dynamic, Scope};

use args::{Args, Command};
use simulation::Simulation;

mod args;
mod engine;
mod helper;
mod maze;
mod mouse;
mod ray;
mod simulation;

#[macroquad::main("Micromouse Simulation")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::ExampleScript => println!("{}", include_str!("../test_data/test.rhai")),
        Command::ExampleMouse => println!("{}", include_str!("../test_data/mouse.toml")),
        Command::ExampleMaze => println!("{}", include_str!("../test_data/example.maze")),
        Command::Simulate {
            maze,
            mouse,
            script,
        } => {
            let maze = std::fs::read_to_string(maze).unwrap();
            let maze: Maze = maze.parse().map_err(|e| anyhow::anyhow!("{}", e))?;

            let mouse_config: MouseConfig =
                toml::from_str(&std::fs::read_to_string(mouse).unwrap()).unwrap();

            let mut sim = Simulation::new(script, maze, mouse_config); // Create a 10x10 maze

            let mut paused = true;

            // Update the simulation
            sim.update(0.0);

            let mut scope = Scope::new();
            scope.push_dynamic("state", Dynamic::from_map(Default::default()));

            loop {
                if is_key_pressed(KeyCode::Space) {
                    paused = !paused;
                }

                let dt = get_frame_time();
                if !paused && !sim.collided {
                    let mut mouse_data = sim.mouse.get_data(dt, sim.collided);
                    scope.push("mouse", mouse_data);

                    sim.engine.run_ast_with_scope(&mut scope, &sim.ast).unwrap();

                    mouse_data = scope.get_value("mouse").unwrap();
                    sim.mouse.update_from_data(mouse_data);

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
    }
    Ok(())
}
