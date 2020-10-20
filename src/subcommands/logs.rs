use crate::user_config::{rules::actions::ActionType, UserConfig};
use chrono::prelude::Local;
use clap::ArgMatches;
use colored::{ColoredString, Colorize};
use regex::Regex;
use std::{
    fs,
    fs::OpenOptions,
    io::{Result, Write},
    path::PathBuf,
};

pub fn logs(args: &ArgMatches) -> Result<()> {
    let logger = Logger::default();
    if args.subcommand().unwrap().1.is_present("clear") {
        logger.delete()
    } else {
        logger.show_logs()
    }
}

pub enum Level {
    Debug,
    Warn,
    Info,
    Error,
}

impl From<&str> for Level {
    fn from(level: &str) -> Self {
        let level = level.to_lowercase();
        match level.as_str() {
            "debug" => Self::Debug,
            "error" => Self::Error,
            "warn" => Self::Warn,
            "info" => Self::Info,
            _ => panic!("unknown log level"),
        }
    }
}

impl ToString for Level {
    fn to_string(&self) -> String {
        match self {
            Self::Debug => "DEBUG",
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
        }
        .to_string()
    }
}

impl Level {
    pub fn colored(&self) -> ColoredString {
        match self {
            Level::Info => self.to_string().green(),
            Level::Error => self.to_string().red(),
            Level::Warn => self.to_string().yellow(),
            Level::Debug => self.to_string().cyan(),
        }
    }
}

pub struct Logger {
    path: PathBuf,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(Self::default_path())
    }
}

impl Logger {
    pub fn default_path() -> PathBuf {
        UserConfig::dir().join(".log")
    }

    pub fn new(path: PathBuf) -> Self {
        OpenOptions::new().append(true).create_new(true).open(&path).ok();
        Self {
            path,
        }
    }

    pub fn try_write(&mut self, level: Level, action: ActionType, msg: &str) {
        if let Err(e) = self.write(level, action, msg) {
            eprintln!("could not write to file: {}", e);
        }
    }

    pub fn write(&mut self, level: Level, action: ActionType, msg: &str) -> Result<()> {
        let datetime = Local::now();
        let level = level.to_string().to_uppercase();
        let file = OpenOptions::new().append(true).open(&self.path)?;
        writeln!(
            &file,
            "[{}-{}] {}: ({}) {}",
            datetime.date(),
            datetime.time(),
            level,
            action.to_string(),
            msg
        )
    }

    pub fn len(&self) -> usize {
        self.read_lines().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn show_logs(&self) -> Result<()> {
        let text = self.read()?;
        let re = Regex::new(r"(?P<time>\[.+]) (?P<level>[A-Z]+?): \((?P<action>\w+?)\) (?P<old_path>.+?) (?:(?P<sep>->) (?P<new_path>.+))?").unwrap();
        for r#match in re.captures_iter(&text) {
            let time = r#match.name("time").unwrap().as_str().dimmed();
            let level = Level::from(r#match.name("level").unwrap().as_str()).colored();
            let action = r#match.name("action").unwrap().as_str().bold();
            let old_path = r#match.name("old_path").unwrap().as_str().underline();
            print!("{} {}: ({}) {}", time, level, action, old_path);

            if let (Some(sep), Some(new_path)) = (r#match.name("sep"), r#match.name("new_path")) {
                println!(" {} {}", sep.as_str(), new_path.as_str().underline())
            } else {
                println!()
            }
        }
        Ok(())
    }

    pub fn delete(self) -> Result<()> {
        fs::remove_file(self.path)
    }

    pub fn read_lines(&self) -> Result<Vec<String>> {
        let logs = fs::read_to_string(&self.path)?;
        Ok(logs.lines().map(|str| str.to_string()).collect::<Vec<_>>())
    }

    pub fn read(&self) -> Result<String> {
        fs::read_to_string(&self.path)
    }
}