use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Clone)]
pub enum Command {
    ExampleMouse,
    ExampleMaze,
    ExampleScript,
    Simulate {
        #[arg(long)]
        maze: Option<PathBuf>,
        #[arg(long)]
        mouse: Option<PathBuf>,
        #[arg(long)]
        script: Option<PathBuf>,
    },
}
