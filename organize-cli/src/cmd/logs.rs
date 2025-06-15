use chrono::Local;
use std::path::Path;
use tracing::Level;
use tracing_subscriber::{
	Layer,
	filter::LevelFilter,
	fmt::{self},
	layer::SubscriberExt,
	util::SubscriberInitExt,
};

/// Initializes the logging system based on command-line arguments and configuration.
///
/// # Arguments
///
/// * `verbose`: A boolean indicating whether to enable verbose (DEBUG) logging to stdout.
/// * `config_path`: The path to the configuration file, used to determine the log directory.
pub fn init<T: AsRef<Path>>(verbose: bool, config_path: T) {
	// 1. Determine the destination directory for logs.
	//    It will be a `.logs` folder in the same directory as the config file.
	let logs_dir = config_path.as_ref().parent().unwrap_or_else(|| Path::new(".")).join("logs");
	std::fs::create_dir_all(&logs_dir).expect("Could not create logs directory");

	// 2. Create a non-blocking file appender for the current run.
	//    The log file will be named with the current timestamp.
	let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
	let log_file = logs_dir.join(format!("{}.log", timestamp));
	let file_appender = tracing_appender::rolling::never(&logs_dir, log_file);
	let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

	// 3. Define the two logging layers.

	// The FILE layer:
	// - Always logs at DEBUG level.
	// - Formats logs as plain text.
	// - Writes to the timestamped file.
	let file_layer = fmt::layer()
		.with_writer(non_blocking_writer)
		.with_ansi(false) // No colors in the log file
		.with_filter(LevelFilter::DEBUG);

	// The STDOUT layer:
	// - Logs at INFO level by default, or DEBUG if `verbose` is true.
	// - Formats logs with colors for readability.
	// - Writes to standard output.
	let stdout_log_level = if verbose { Level::DEBUG } else { Level::INFO };
	let stdout_layer = fmt::layer()
		.with_writer(std::io::stdout)
		.with_filter(LevelFilter::from_level(stdout_log_level));

	// 4. Combine the layers and initialize the global subscriber.
	//    The `_guard` from the non-blocking writer must be kept in scope for the
	//    duration of the program, so we leak it.
	tracing_subscriber::registry().with(file_layer).with(stdout_layer).init();
	std::mem::forget(_guard);

	tracing::debug!("Logging initialized. Log file: {}", logs_dir.display());
}
