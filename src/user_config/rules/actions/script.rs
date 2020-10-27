use crate::{
    string::Placeholder,
    user_config::{rules::filters::AsFilter, UserConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Result,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    str::FromStr,
};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Script {
    exec: String,
    content: String,
}

impl AsFilter for Script {
    fn matches(&self, path: &Path) -> bool {
        let output = self.run_as_action(path).unwrap().stdout;
        let output = String::from_utf8_lossy(&output);
        let parsed = bool::from_str(&output.trim().to_lowercase());
        println!("{:?}", parsed);
        match parsed {
            Ok(boolean) => boolean,
            Err(_) => false,
        }
    }
}

impl Script {
    pub fn write(&self, path: &Path) -> Result<PathBuf> {
        let content = self.content.expand_placeholders(path)?;
        let dir = UserConfig::dir().join("scripts");
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let script = dir.join("temp_script");
        fs::write(&script, content)?;
        Ok(script)
    }

    pub fn run_as_action(&self, path: &Path) -> Result<Output> {
        let script = self.write(path)?;
        let output = Command::new(&self.exec)
            .arg(&script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("could not run script")
            .wait_with_output()
            .expect("script terminated with an error");
        fs::remove_file(script)?;
        Ok(output)
    }
}
// #[cfg(test)]
// mod tests {
//     use crate::{
//         path::IsHidden,
//         user_config::rules::{actions::script::Script, filters::Filters},
//     };
//     use std::{
//         io::{Error, ErrorKind, Result},
//         path::PathBuf,
//     };
//
//     #[test]
//     fn check_filter_python() -> Result<()> {
//         let substr = "Downloads";
//         let mut filters = Filters::default();
//         let script = Script {
//             exec: "python".into(),
//             content: format!("'{}' in str('{{path}}')", substr),
//         };
//         filters.script = Some(script);
//         let path = PathBuf::from("$HOME/Downloads/test.pdf");
//         if path.matches_filters(&filters) {
//             Ok(())
//         } else {
//             Err(Error::from(ErrorKind::Other))
//         }
//     }
// }
