use anyhow::Result;
use clap::Clap;
use log::Level;

use organize_core::logger::Logger;

use crate::Cmd;

#[derive(Debug, Clap)]
pub struct Logs {
	#[clap(long, about = "Do not print colored output")]
	pub(crate) no_color: bool,
}

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
