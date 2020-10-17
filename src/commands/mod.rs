pub mod config;
pub mod run;
pub mod stop;
pub mod watch;

#[derive(Clone, PartialEq, Debug)]
pub enum SubCommands {
    Config,
    Run,
    Suggest,
    Watch,
    Logs,
    Stop,
}
