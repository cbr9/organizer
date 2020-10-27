use crate::{user_config::rules::actions::ActionType, ARGS, LOG_FILE};
use colored::Colorize;
use std::ops::Deref;
use std::{fs, io::Result};

pub struct LogMessage<'a> {
    pub(crate) action: &'a ActionType,
    pub(crate) message: String,
}

impl<'a> LogMessage<'a> {
    pub fn new<T: Into<String>>(action: &'a ActionType, message: T) -> Self {
        Self {
            action,
            message: message.into(),
        }
    }
}

impl<'a> ToString for LogMessage<'a> {
    fn to_string(&self) -> String {
        format!("({}) {}", self.action.to_string().bold(), self.message)
    }
}

pub fn logs() -> Result<()> {
    if ARGS.is_present("clear") {
        fs::remove_file(LOG_FILE.deref())
    } else {
        let text = fs::read_to_string(LOG_FILE.deref())?;
        for line in text.lines() {
            println!("{}", line);
        }
        Ok(())
    }
}
