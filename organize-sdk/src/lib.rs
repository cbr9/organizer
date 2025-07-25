#![feature(file_lock)]
#![feature(path_add_extension)]
#![feature(lock_value_accessors)]

pub const PROJECT_NAME: &str = "organize";

pub mod context;
pub mod engine;
pub mod error;
pub mod location;
pub mod plugins;
pub mod stdx;
pub mod templates;
pub mod utils;
