use crate::{
    user_config::{rules::actions::ActionType, UserConfig},
    ARGS,
};
use chrono::prelude::Local;
use colored::{ColoredString, Colorize};
use regex::Regex;
use std::{
    fs,
    fs::OpenOptions,
    io::{Result, Write},
    path::PathBuf,
    result,
    str::FromStr,
};

pub fn logs() -> Result<()> {
    let logger = Logger::default();
    if ARGS.is_present("clear") {
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

impl FromStr for Level {
    type Err = ();

    fn from_str(level: &str) -> result::Result<Self, Self::Err> {
        let level = level.to_lowercase();
        match level.as_str() {
            "debug" => Ok(Self::Debug),
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
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

struct Line {
    time: ColoredString,
    level: ColoredString,
    action: ColoredString,
    old_path: ColoredString,
    sep: Option<String>,
    new_path: Option<ColoredString>,
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
        OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(&path)
            .ok();
        Self { path }
    }

    pub fn try_write(&mut self, level: &Level, action: &ActionType, msg: &str) {
        if let Err(e) = self.write(level, action, msg) {
            eprintln!("could not write to file: {}", e);
        }
    }

    pub fn write(&mut self, level: &Level, action: &ActionType, msg: &str) -> Result<()> {
        let datetime = Local::now();
        let level = level.to_string().to_uppercase();
        let file = OpenOptions::new().append(true).open(&self.path)?;
        let msg = format!(
            "[{}-{}] {}: ({}) {}",
            datetime.date(),
            datetime.time(),
            level,
            action.to_string(),
            msg
        );
        let Line {
            time,
            level,
            action,
            old_path,
            sep,
            new_path,
        } = Self::format(&msg);
        let mut msg = format!("{} {}: ({}) {}", time, level, action, old_path);
        if let (Some(sep), Some(new_path)) = (sep, new_path) {
            msg.push_str(&format!(" {} {}", sep, new_path));
        }
        println!("{}", msg);
        writeln!(&file, "{}", msg)
    }

    fn format(line: &str) -> Line {
        let re = Regex::new(r"(?P<time>\[.+]) (?P<level>[A-Z]+?): \((?P<action>\w+?)\) (?P<old_path>.+?) (?:(?P<sep>->) (?P<new_path>.+))?").unwrap();
        let r#match = re.captures(line).unwrap();
        let mut line = Line {
            time: r#match.name("time").unwrap().as_str().dimmed(),
            level: r#match
                .name("level")
                .unwrap()
                .as_str()
                .parse::<Level>()
                .unwrap()
                .colored(),
            action: r#match.name("action").unwrap().as_str().bold(),
            old_path: r#match.name("old_path").unwrap().as_str().underline(),
            sep: None,
            new_path: None,
        };
        if let (Some(sep), Some(new_path)) = (r#match.name("sep"), r#match.name("new_path")) {
            line.sep = Some(sep.as_str().to_string());
            line.new_path = Some(new_path.as_str().underline());
        }
        line
    }

    pub fn show_logs(&self) -> Result<()> {
        let text = self.read()?;
        for line in text.lines() {
            println!("{}", line);
        }
        Ok(())
    }

    pub fn delete(self) -> Result<()> {
        fs::remove_file(self.path)
    }

    pub fn read(&self) -> Result<String> {
        fs::read_to_string(&self.path)
    }
}
