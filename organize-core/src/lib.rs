#![feature(file_lock)]
#![feature(path_add_extension)]
#![feature(lock_value_accessors)]

pub const PROJECT_NAME: &str = "organize";

pub mod action;
pub mod batch;
pub mod builtins;
pub mod common;
pub mod config;
pub mod context;
pub mod engine;
pub mod errors;
pub mod filter;
pub mod folder;
pub mod grouper;
pub mod selector;
pub mod splitter;
// pub mod hook;
pub mod options;
pub mod parser;
pub mod pipeline;
pub mod resource;
pub mod rule;
pub mod sorter;
pub mod stdx;
pub mod storage;
pub mod templates;
pub mod utils;
