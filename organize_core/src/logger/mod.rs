use std::fmt::Arguments;
use std::fmt::Display;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::format::{DelayedFormat, StrftimeItems};
use chrono::{Local, NaiveDateTime};
use colored::Colorize;
use fern::colors::{Color, ColoredLevelConfig};
use fern::{Dispatch, FormatCallback, Output};
use lazy_static::lazy_static;
use log::{Level, Record};
use regex::Regex;

use crate::data::Data;

lazy_static! {
	static ref COLORS: ColoredLevelConfig = Logger::colors();
	static ref TIME_FORMAT: &'static str = "[%F][%T]";
}

pub struct Log {
	timestamp: NaiveDateTime,
	level: Level,
	message: String,
}

lazy_static! {
	pub static ref LOG_PATTERN: Regex =
		Regex::new(r"(?P<timestamp>\[\d{4}?-\d{2}-\d{2}]\[\d{2}:\d{2}:\d{2}]) (?P<level>INFO|DEBUG|WARN|ERROR|TRACE): (?P<message>.+$)").unwrap();
}

fn format<T: Display, Q: Display, P: Display>(timestamp: T, level: Q, message: P) -> String {
	format!("{timestamp} {level}: {message}", timestamp = timestamp, level = level, message = message)
}

impl<T: AsRef<str>> From<T> for Log {
	fn from(s: T) -> Self {
		let s = s.as_ref();
		let groups = LOG_PATTERN.captures(s).expect("invalid log format");
		let timestamp = groups.name("timestamp").expect("invalid time format").as_str();
		let level = groups.name("level").expect("invalid level").as_str();
		let message = groups.name("message").unwrap().as_str();

		Log {
			timestamp: NaiveDateTime::parse_from_str(timestamp, *TIME_FORMAT).unwrap(),
			level: Level::from_str(level).unwrap(),
			message: message.to_string(),
		}
	}
}

impl Log {
	pub fn colored(self) -> String {
		format(
			self.timestamp.format(*TIME_FORMAT).to_string().dimmed(),
			COLORS.color(self.level),
			self.message,
		)
	}
	pub fn plain(self) -> String {
		format(self.timestamp.format(*TIME_FORMAT), self.level, self.message)
	}
}

pub struct Logger;

impl Logger {
	fn time() -> DelayedFormat<StrftimeItems<'static>> {
		Local::now().format(*TIME_FORMAT)
	}

	fn colors() -> ColoredLevelConfig {
		ColoredLevelConfig::new()
			.info(Color::BrightGreen)
			.warn(Color::BrightYellow)
			.error(Color::BrightRed)
	}

	pub fn parse(level: Level) -> anyhow::Result<Vec<Log>> {
		Self::path(level).map(|path| Ok(std::fs::read_to_string(path)?.lines().map(Log::from).collect()))?
	}

	fn plain_format(out: FormatCallback, message: &Arguments, record: &Record) {
		out.finish(format_args!("{}", format(Self::time(), record.level(), message)))
	}

	fn colored_format(out: FormatCallback, message: &Arguments, record: &Record) {
		out.finish(format_args!(
			"{}",
			format(Self::time().to_string().dimmed(), COLORS.color(record.level()), message)
		))
	}

	fn path(level: Level) -> anyhow::Result<PathBuf> {
		let dir = Data::dir()?.join("logs");
		match level {
			Level::Error | Level::Warn => Ok(dir.join("errors.log")),
			Level::Info => Ok(dir.join("output.log")),
			Level::Debug | Level::Trace => Ok(dir.join("debug.log")),
		}
	}

	fn build_dispatchers<T: Into<Output> + Write>(level: Level, no_color: bool, writer: T) -> anyhow::Result<(Dispatch, Dispatch)> {
		let console_output = fern::Dispatch::new()
			.filter(move |metadata| metadata.level() == level)
			.format(move |out, args, record| {
				if no_color {
					Self::plain_format(out, args, record)
				} else {
					Self::colored_format(out, args, record);
				}
			})
			.chain(writer);

		let file = Self::path(level).map(|path| -> anyhow::Result<Dispatch> {
			match path.parent() {
				None => return Err(anyhow::Error::msg("could not determine parent directory")),
				Some(parent) => {
					if !parent.exists() {
						std::fs::create_dir_all(&parent)?;
					}
				}
			}
			Ok(fern::Dispatch::new()
				.filter(move |metadata| metadata.level() == level)
				.format(Self::plain_format) // we don't want ANSI escape codes to be written to the log file
				.chain(fern::log_file(path)?))
		})??;

		Ok((console_output, file))
	}

	pub fn setup(no_color: bool) -> Result<(), anyhow::Error> {
		let info = Self::build_dispatchers(Level::Info, no_color, std::io::stdout())?;
		let debug = Self::build_dispatchers(Level::Debug, no_color, std::io::stdout())?;
		let error = Self::build_dispatchers(Level::Error, no_color, std::io::stderr())?;
		let warn = Self::build_dispatchers(Level::Warn, no_color, std::io::stderr())?;

		fern::Dispatch::new()
			.chain(info.0)
			.chain(info.1)
			.chain(debug.0)
			.chain(error.0)
			.chain(error.1)
			.chain(warn.0)
			.chain(warn.1)
			.apply()?;

		Ok(())
	}
}
