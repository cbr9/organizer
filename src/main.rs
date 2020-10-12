pub mod cli;
pub mod configuration;
pub mod file;
pub mod lock_file;
pub mod subcommands;

use crate::{
    cli::Cli,
    subcommands::edit::UserConfig,
};
use clap::crate_name;
use colored::Colorize;
use std::io::Error;

static PROJECT_NAME: &str = crate_name!();

fn main() -> Result<(), Error> {
    let mut cli = Cli::new();
    match cli.run() {
        Ok(_) => {}
        Err(e) => eprintln!("{}", e),
    }
    Ok(())
}
