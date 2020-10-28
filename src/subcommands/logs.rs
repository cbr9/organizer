use crate::{ARGS, LOG_FILE};
use std::{fs, io::Result, ops::Deref};

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
