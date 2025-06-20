#![feature(file_lock)]
#![feature(path_add_extension)]
#![feature(lock_value_accessors)]

pub const PROJECT_NAME: &str = "organize";

pub mod config;
pub mod engine;
pub mod errors;
pub mod path;
pub mod resource;
pub mod templates;
pub mod utils;
