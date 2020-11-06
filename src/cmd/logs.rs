use crate::{cmd::Cmd, user_config::UserConfig};
use anyhow::Result;
use clap::Clap;
use std::{fs, path::PathBuf};

#[derive(Debug, Clap)]
pub struct Logs {
	#[clap(long)]
	clear: bool,
}

impl Cmd for Logs {
	fn run(self) -> Result<()> {
		if self.clear {
			fs::remove_file(Self::path()).map_err(anyhow::Error::new)
		} else {
			let text = fs::read_to_string(Self::path())?;
			for line in text.lines() {
				println!("{}", line);
			}
			Ok(())
		}
	}
}

impl Logs {
	pub fn path() -> PathBuf {
		UserConfig::dir().join("output.log")
	}
}
