use anyhow::Result;
use clap::Parser;
use log::Level;

use organize_core::logger::Logger;

use crate::Cmd;

#[derive(Debug, Parser)]
pub struct Logs {
	#[arg(long)]
	pub(crate) no_color: bool,
}

// TODO: Add options to show errors and debug messages
impl Cmd for Logs {
	fn run(self) -> Result<()> {
		let logs = Logger::parse(Level::Info)?;
		if self.no_color {
			logs.into_iter().map(|log| log.plain()).for_each(|line| println!("{}", line))
		} else {
			logs.into_iter().map(|log| log.colored()).for_each(|line| println!("{}", line))
		};

		Ok(())
	}
}
