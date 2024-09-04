use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone)]
pub enum Command {
    ExampleMouse,
    ExampleMaze,
    ExampleScript,
    Simulate {
        maze: PathBuf,
        mouse: PathBuf,
        script: PathBuf,
    },
}
