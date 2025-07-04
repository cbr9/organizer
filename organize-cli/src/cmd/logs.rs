use chrono::Local;
use clap::ValueEnum;
use std::path::PathBuf;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard; // Import the guard type
use tracing_subscriber::{
	Layer,
	filter::LevelFilter,
	fmt::{self},
	layer::SubscriberExt,
	util::SubscriberInitExt,
};

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum LogLevel {
	#[default]
	Info,
	Debug,
	Trace,
	Warn,
	Error,
}

// Implement a conversion from our CLI enum to the `tracing` LevelFilter.
impl From<LogLevel> for Level {
	fn from(level: LogLevel) -> Self {
		match level {
			LogLevel::Info => Level::INFO,
			LogLevel::Debug => Level::DEBUG,
			LogLevel::Trace => Level::TRACE,
			LogLevel::Warn => Level::WARN,
			LogLevel::Error => Level::ERROR,
		}
	}
}

/// Initializes the logging system and returns a guard that must be kept in scope.
pub fn init(level: LogLevel) -> WorkerGuard {
	// 1. Determine the destination directory for logs.
	let logs_dir = PathBuf::from(".").join("logs"); // A hidden folder is a common convention

	// 2. Create a non-blocking file appender for the current run.
	// We add milliseconds to the timestamp to increase uniqueness.
	let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S%.3f");
	let log_file = format!("{timestamp}.log");
	let file_appender = tracing_appender::rolling::never(&logs_dir, log_file);
	let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);

	// 3. Define the two logging layers.
	let file_layer = fmt::layer()
		.with_writer(non_blocking_writer)
		.with_ansi(false)
		.pretty()
		.with_filter(LevelFilter::TRACE);

	// 4. Combine the layers and initialize the global subscriber.
	tracing_subscriber::registry().with(file_layer).init();

	tracing::debug!("Logging initialized. Log file in: {}", logs_dir.display());

	// 5. Return the guard to the caller.
	guard
}
