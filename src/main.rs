use crate::{
    lock_file::LockFile,
    subcommands::{config::config, logs::logs, run::run, stop::stop, watch::watch},
    user_config::UserConfig,
};
use clap::{
    crate_authors,
    crate_description,
    crate_name,
    crate_version,
    load_yaml,
    App,
    ArgMatches,
};
use colored::Colorize;
use fern::colors::{Color, ColoredLevelConfig};
use lazy_static::lazy_static;
use std::{env, io::Error, path::PathBuf};

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
    pub static ref CONFIG: UserConfig = UserConfig::default();
    pub static ref LOCK_FILE: LockFile = LockFile::new();
    pub static ref LOG_FILE: PathBuf = UserConfig::dir().join("output.log");
}

fn main() -> Result<(), Error> {
    debug_assert!(MATCHES.subcommand().is_some());
    setup_logger().unwrap();

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

fn setup_logger() -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .info(Color::BrightGreen)
        .warn(Color::BrightYellow)
        .error(Color::BrightRed);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} {}: {}",
                chrono::Local::now()
                    .format("[%Y-%m-%d][%H:%M:%S]")
                    .to_string()
                    .dimmed(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file(UserConfig::dir().join("output.log"))?)
        .apply()?;
    Ok(())
}
