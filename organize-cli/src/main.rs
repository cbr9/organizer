use crate::cmd::{Cmd, OrganizeCli};
use anyhow::Result;
use clap::Parser;

mod cli;
mod cmd;

#[allow(unused_imports)]
use organize_std::*;

#[tokio::main]
async fn main() -> Result<()> {
	let app: OrganizeCli = OrganizeCli::parse();
	app.run().await
}
