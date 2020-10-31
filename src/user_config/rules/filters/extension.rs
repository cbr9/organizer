use crate::user_config::rules::filters::AsFilter;
use serde::{
    de,
    de::{SeqAccess, Visitor},
    export,
    export::PhantomData,
    Deserialize,
    Deserializer,
};
use std::{fmt, ops::Deref, path::Path};

#[derive(Debug, Clone)]
pub struct Extension(Vec<String>);

impl Deref for Extension {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Extension {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrSeq(PhantomData<fn() -> Extension>);

        impl<'de> Visitor<'de> for StringOrSeq {
            type Value = Extension;

            fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
                formatter.write_str("string or seq")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Extension(vec![value.into()]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(val) = seq.next_element()? {
                    vec.push(val)
                }
                Ok(Extension(vec))
            }
        }

        deserializer.deserialize_any(StringOrSeq(PhantomData))
    }
}

impl AsFilter for Extension {
    fn matches(&self, path: &Path) -> bool {
        match path.extension() {
            Some(extension) => self.contains(&extension.to_str().unwrap().to_string()),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Extension;
    use crate::user_config::rules::filters::AsFilter;
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
        let path = PathBuf::from("$HOME/Downloads/test.pdf");
        match extension.matches(&path) {
            true => Ok(()),
            false => Err(Error::from(ErrorKind::Other)),
        }
    }

    #[test]
    fn multiple_match_pdf() -> Result<(), Error> {
        let extension = Extension(vec!["pdf".into(), "doc".into(), "docx".into()]);
        let path = PathBuf::from("$HOME/Downloads/test.pdf");
        match extension.matches(&path) {
            true => Ok(()),
            false => Err(Error::from(ErrorKind::Other)),
        }
    }
}
