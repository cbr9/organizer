use regex::Regex;
use std::{
    borrow::Cow,
    io::{Error, ErrorKind, Result},
    path::Path,
};

pub trait Capitalize<T> {
    fn capitalize(&self) -> T;
}

pub trait Placeholder {
    fn expand_placeholders(&self, path: &Path) -> Result<Cow<'_, str>>;
    fn placeholder_error(placeholder: &str, current_value: &Path, span: &str) -> Error;
}

impl Capitalize<String> for String {
    fn capitalize(&self) -> String {
        if self.is_empty() {
            return self.clone();
        }
        let mut c = self.chars();
        c.next().unwrap().to_uppercase().collect::<String>() + c.as_str()
    }
}

impl Placeholder for &str {
    fn expand_placeholders(&self, path: &Path) -> Result<Cow<'_, str>> {
        // TODO: check invalid placeholders like {stem.extension} or {extension.path}
        let regex = Regex::new(r"\{\w+(?:\.\w+)*}").unwrap();
        if regex.is_match(self) {
            let mut new = self.to_string();
            for span in regex.find_iter(self) {
                let placeholders = span
                    .as_str()
                    .trim_matches(|x| x == '{' || x == '}')
                    .split('.');
                let mut current_value = path.to_path_buf();
                for placeholder in placeholders.into_iter() {
                    current_value = match placeholder {
                        "path" => current_value.canonicalize().ok().ok_or_else(|| {
                            Self::placeholder_error(placeholder, &current_value, span.as_str())
                        })?,
                        "parent" => current_value
                            .parent()
                            .ok_or_else(|| {
                                Self::placeholder_error(placeholder, &current_value, span.as_str())
                            })?
                            .into(),
                        "name" => current_value
                            .file_name()
                            .ok_or_else(|| {
                                Self::placeholder_error(placeholder, &current_value, span.as_str())
                            })?
                            .into(),
                        "stem" => current_value
                            .file_stem()
                            .ok_or_else(|| {
                                Self::placeholder_error(placeholder, &current_value, span.as_str())
                            })?
                            .into(),
                        "extension" => current_value
                            .extension()
                            .ok_or_else(|| {
                                Self::placeholder_error(placeholder, &current_value, span.as_str())
                            })?
                            .into(),
                        "to_uppercase" => current_value.to_str().unwrap().to_uppercase().into(),
                        "to_lowercase" => current_value.to_str().unwrap().to_lowercase().into(),
                        "capitalize" => current_value
                            .to_str()
                            .unwrap()
                            .to_string()
                            .capitalize()
                            .into(),
                        _ => panic!("unknown placeholder"),
                    }
                }
                new = new.replace(&span.as_str(), current_value.to_str().unwrap());
            }
            Ok(Cow::Owned(new.replace("//", "/")))
        } else {
            Ok(Cow::Borrowed(self))
        }
    }

    fn placeholder_error(placeholder: &str, current_value: &Path, span: &str) -> Error {
        let message = format!(
            "tried to retrieve the {} from {}, but it does not contain it (placeholder: {})",
            placeholder,
            current_value.display(),
            span
        );
        Error::new(ErrorKind::Other, message)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        path::Expandable,
        string::{Capitalize, Placeholder},
    };
    use std::{
        borrow::Cow,
        io::{Error, ErrorKind, Result},
        path::{Path, PathBuf},
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

    #[test]
    fn multiple_placeholders_sentence() -> Result<()> {
        let tested = "To run this program, you have to change directory into $HOME/{extension}/{parent.name}";
        let path = PathBuf::from("$HOME/Documents/test.pdf").expand_vars();
        let new_path = tested.expand_placeholders(&path).unwrap();
        let expected = String::from(
            "To run this program, you have to change directory into $HOME/pdf/Documents",
        );
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }

    #[test]
    fn no_placeholder() -> Result<()> {
        let tested = "/home/cabero/Documents/test.pdf";
        let dummy_path = PathBuf::from(tested);
        let new = tested.expand_placeholders(&dummy_path)?;
        match new {
            Cow::Borrowed(_) => Ok(()),
            Cow::Owned(_) => Err(Error::from(ErrorKind::Other)),
        }
    }
}
