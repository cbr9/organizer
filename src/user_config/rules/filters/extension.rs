use crate::user_config::rules::{deserialize::string_or_seq, filters::AsFilter};
use serde::Deserialize;
use std::{ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Extension(#[serde(deserialize_with = "string_or_seq")] Vec<String>);

impl Deref for Extension {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsFilter for Extension {
    fn matches(&self, path: &Path) -> bool {
        self.contains(&path.extension().unwrap().to_str().unwrap().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Extension;
    use crate::{path::Expandable, user_config::rules::filters::AsFilter};
    use serde_yaml::Error as YamlError;
    use std::{
        io::{Error, ErrorKind},
        path::PathBuf,
    };

    #[test]
    fn deserialize_string() -> Result<(), YamlError> {
        let extension: Result<Extension, YamlError> = serde_yaml::from_str("pdf");
        match extension {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[test]
    fn deserialize_seq() -> Result<(), YamlError> {
        let extension: Result<Extension, YamlError> = serde_yaml::from_str("[pdf, doc, docx]");
        match extension {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[test]
    fn single_match_pdf() -> Result<(), Error> {
        let extension = Extension(vec!["pdf".into()]);
        let path = PathBuf::from("$HOME/Downloads/test.pdf").expand_vars();
        match extension.matches(&path) {
            true => Ok(()),
            false => Err(Error::from(ErrorKind::Other)),
        }
    }

    #[test]
    fn multiple_match_pdf() -> Result<(), Error> {
        let extension = Extension(vec!["pdf".into(), "doc".into(), "docx".into()]);
        let path = PathBuf::from("$HOME/Downloads/test.pdf").expand_vars();
        match extension.matches(&path) {
            true => Ok(()),
            false => Err(Error::from(ErrorKind::Other)),
        }
    }
}
