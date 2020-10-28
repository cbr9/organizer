#[cfg(test)]
mod tests {
    use crate::string::{Capitalize, Placeholder};
    use std::{
        io::{Error, ErrorKind, Result},
        path::Path,
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
    #[test]
    fn single_placeholder() -> Result<()> {
        let tested = "/home/cabero/Downloads/{parent.name}";
        let new_path = tested
            .expand_placeholders(&Path::new("/home/cabero/Documents/test.pdf"))
            .unwrap();
        let expected = String::from("/home/cabero/Downloads/Documents");
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }
    #[test]
    fn multiple_placeholders() -> Result<()> {
        let tested = "/home/cabero/{extension}/{parent.name}";
        let new_path = tested
            .expand_placeholders(&Path::new("/home/cabero/Documents/test.pdf"))
            .unwrap();
        let expected = String::from("/home/cabero/pdf/Documents");
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }
}
