use std::fmt::Arguments;
use std::fmt::Display;
use std::iter::Map;
use std::path::PathBuf;
use std::str::{FromStr, Lines};

use chrono::{Local, NaiveDateTime};
use chrono::format::{DelayedFormat, StrftimeItems};
use colored::Colorize;
use fern::{Dispatch, FormatCallback};
use fern::colors::{Color, ColoredLevelConfig};
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

	pub fn parse(level: Level) -> std::io::Result<Vec<Log>> {
		use Level::*;
		let path = match level {
			Error | Debug | Warn => Self::debug(),
			Info => Self::actions(),
			Trace => unreachable!(),
		};
		Ok(std::fs::read_to_string(path)?.lines().map(Log::from).collect())
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

	pub fn actions() -> PathBuf {
		Data::dir().join("output.log")
	}

	fn debug() -> PathBuf {
		Data::dir().join("debug.log")
	}

	pub fn setup(no_color: bool) -> Result<(), anyhow::Error> {
		let config = fern::Dispatch::new();
		let build_dispatchers = move |level: Level, no_color: bool, path: PathBuf| -> std::io::Result<(Dispatch, Dispatch)> {
			let stdout = fern::Dispatch::new()
				.filter(move |metadata| metadata.level() == level)
				.format(move |out, args, record| {
					if no_color {
						Self::plain_format(out, args, record)
					} else {
						Self::colored_format(out, args, record);
					}
				})
				.chain(std::io::stdout());
			let file = fern::Dispatch::new()
				.filter(move |metadata| metadata.level() == level)
				.format(Self::plain_format) // we don't want ANSI escape codes to be written to the log file
				.chain(fern::log_file(path)?);

			Ok((stdout, file))
		};
		let info = build_dispatchers(Level::Info, no_color, Self::actions())?;
		let debug = build_dispatchers(Level::Debug, no_color, Self::debug())?;
		let error = build_dispatchers(Level::Error, no_color, Self::debug())?;
		let warn = build_dispatchers(Level::Warn, no_color, Self::debug())?;

		config
			.chain(debug.0)
			.chain(debug.1)
			.chain(info.0)
			.chain(info.1)
			.chain(error.0)
			.chain(error.1)
			.chain(warn.0)
			.chain(warn.1)
			.apply()?;

		Ok(())
	}
}
