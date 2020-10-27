use crate::{
    lock_file::LockFile,
    subcommands::{config::config, logs::logs, run::run, stop::stop, watch::watch},
    user_config::UserConfig,
};
use clap::{
    crate_authors, crate_description, crate_name, crate_version, load_yaml, App, ArgMatches,
};
use lazy_static::lazy_static;
use std::{env, io::Result};

pub mod lock_file;
pub mod path;
pub mod string;
pub mod subcommands;
pub mod user_config;

lazy_static! {
    pub static ref MATCHES: ArgMatches = App::from(load_yaml!("cli.yml"))
        .author(crate_authors!())
        .about(crate_description!())
        .version(crate_version!())
        .name(crate_name!())
        .get_matches();
    pub static ref ARGS: &'static ArgMatches = MATCHES.subcommand().unwrap().1;
    pub static ref CONFIG: UserConfig = UserConfig::new();
    pub static ref LOCK_FILE: LockFile = LockFile::new();
}

fn main() -> Result<()> {
    debug_assert!(MATCHES.subcommand().is_some());

    if cfg!(target_os = "windows") {
        eprintln!("Windows is not supported yet");
        return Ok(());
    }

    match MATCHES.subcommand_name().unwrap() {
        "config" => config(),
        "run" => run(),
        "watch" => watch(),
        "stop" => stop(),
        "logs" => logs(),
        _ => panic!("unknown subcommand"),
    }
}
