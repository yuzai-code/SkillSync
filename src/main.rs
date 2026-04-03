mod cli;
mod claude;
mod installer;
mod registry;
mod state;
mod tui;
mod watcher;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::run(cli)
}
