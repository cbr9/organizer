#![feature(file_lock)]
#![feature(path_add_extension)]

pub const PROJECT_NAME: &str = "organize";

pub mod config;
pub mod engine;
pub mod path;
pub mod resource;
pub mod templates;
