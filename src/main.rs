mod cli;
mod claude;
pub mod i18n;
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
