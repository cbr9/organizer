use std::{
    env,
    path::{Path, PathBuf},
};

pub trait Expandable {
    fn expand_user(self) -> PathBuf;
    fn expand_vars(self) -> PathBuf;
}

impl Expandable for PathBuf {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn expand_user(self) -> PathBuf {
        let str = self.to_str().unwrap();
        if str.contains('~') {
            match env::var("HOME") {
                Ok(home) => {
                    let new = str.replace("~", &home);
                    new.into()
                }
                Err(e) => panic!("error: {}", e),
            }
        } else {
            self
        }
    }

    fn expand_vars(self) -> PathBuf {
        // TODO: avoid panic, return a serde error
        if self.to_string_lossy().contains('$') {
            self.components()
                .map(|component| {
                    let component: &Path = component.as_ref();
                    let component = component.to_string_lossy();
                    if component.starts_with('$') {
                        env::var(component.replace('$', "")).unwrap_or_else(|_| {
                            panic!(
                                "error: environment variable '{}' could not be found",
                                component
                            )
                        })
                    } else {
                        component.to_string()
                    }
                })
                .collect::<PathBuf>()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests::{project, IntoResult};
    use dirs::home_dir;
    use std::{env, io::Result};

    #[test]
    fn home() -> Result<()> {
        let original = PathBuf::from("$HOME/Documents");
        let expected = home_dir().unwrap().join("Documents");
        (original.expand_vars() == expected).into_result()
    }
    #[test]
    fn new_var() -> Result<()> {
        env::set_var("PROJECT_DIR", project());
        let original = PathBuf::from("$PROJECT_DIR/tests");
        (original.expand_vars() == project().join("tests")).into_result()
    }
    #[test]
    #[should_panic]
    fn non_existing_var() {
        let var = "PROJECT_DIR_2";
        let tested = PathBuf::from(format!("${}/tests", var));
        tested.expand_vars();
    }
}
