pub trait Capitalize {
    fn capitalize(&self) -> String;
}

impl Capitalize for String {
    fn capitalize(&self) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        let mut c = self.chars();
        c.next().unwrap().to_uppercase().collect::<String>() + c.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Error, ErrorKind, Result},
    };

    #[test]
    fn capitalize_word() -> Result<()> {
        let tested = String::from("house");
        let expected = String::from("House");
        if tested.capitalize() == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }
    #[test]
    fn capitalize_single_char() -> Result<()> {
        let tested = String::from("h");
        let expected = String::from("H");
        if tested.capitalize() == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }
}
