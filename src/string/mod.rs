use regex::Regex;
use std::{
    borrow::Cow,
    io::{Error, ErrorKind, Result},
    path::Path,
};

mod lib;

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
                        "path" => current_value,
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
            "tried retrieving {} from {}, which does not exist (full placeholder: {})",
            placeholder,
            current_value.display(),
            span
        );
        Error::new(ErrorKind::Other, message)
    }
}
