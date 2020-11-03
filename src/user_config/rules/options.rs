use crate::SETTINGS;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    ops::{Add, Deref},
    path::PathBuf,
};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum Apply {
    All,
    Any,
    Select(Vec<usize>),
}

impl AsRef<Self> for Apply {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl ToString for Apply {
    fn to_string(&self) -> String {
        match self {
            Apply::All => "all".into(),
            Apply::Any => "any".into(),
            Apply::Select(vec) => format!("{:?}", vec),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Options {
    /// defines whether or not subdirectories must be scanned
    pub recursive: Option<bool>,
    pub watch: Option<bool>,
    pub ignore: Option<Vec<PathBuf>>,
    pub hidden_files: Option<bool>,
    pub apply: Option<Apply>,
}

pub fn combine_options<T>(lhs: Option<T>, rhs: Option<T>, default: Option<T>) -> Option<T> {
    match (&lhs, &rhs) {
        (None, Some(_)) => rhs,
        (Some(_), None) => lhs,
        (None, None) => default,
        (Some(_), Some(_)) => rhs,
    }
}

pub fn combine_option_vec<T>(
    lhs: &Option<Vec<T>>,
    rhs: &Option<Vec<T>>,
    default: Option<Vec<T>>,
) -> Option<Vec<T>>
where
    T: Clone,
{
    match (&lhs, &rhs) {
        (None, Some(rhs)) => Some(rhs.clone()),
        (Some(lhs), None) => Some(lhs.clone()),
        (None, None) => default,
        (Some(lhs), Some(rhs)) => {
            let mut rhs = rhs.clone();
            let lhs = &mut lhs.clone();
            rhs.append(lhs);
            Some(rhs)
        }
    }
}

impl Add<Self> for &Options {
    type Output = Options;

    fn add(self, rhs: &Options) -> Self::Output {
        let defaults = &SETTINGS.defaults;
        Options {
            watch: combine_options(self.watch, rhs.watch, defaults.watch),
            recursive: combine_options(self.recursive, rhs.recursive, defaults.recursive),
            hidden_files: combine_options(
                self.hidden_files,
                rhs.hidden_files,
                defaults.hidden_files,
            ),
            apply: combine_options(
                self.apply.clone(),
                rhs.apply.clone(),
                defaults.apply.clone(),
            ),
            ignore: combine_option_vec(&self.ignore, &rhs.ignore, defaults.ignore.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{settings::Settings, utils::tests::IntoResult};
    use std::io::Result;

    #[test]
    fn add_two() -> Result<()> {
        let defaults = Settings::default();
        let opt1 = Options {
            recursive: Some(true),
            watch: None,
            ignore: Some(vec!["$HOME".into(), "$HOME/Downloads".into()]),
            hidden_files: None,
            apply: Some(Apply::All),
        };
        let opt2 = Options {
            recursive: Some(false),
            watch: Some(false),
            ignore: Some(vec!["$HOME/Documents".into()]),
            hidden_files: None,
            apply: Some(Apply::Any),
        };
        let expected = Options {
            recursive: opt2.recursive.clone(),
            watch: opt2.watch.clone(),
            ignore: Some({
                let mut ignore1 = opt1.clone().ignore.unwrap().clone();
                let ignore2 = &mut opt2.clone().ignore.unwrap();
                ignore2.append(&mut ignore1);
                ignore2.clone()
            }),
            hidden_files: defaults.defaults.hidden_files,
            apply: opt2.apply.clone(),
        };
        (&opt1 + &opt2 == expected).into_result()
    }
    #[test]
    fn add_three() -> Result<()> {
        let opt1 = Options {
            recursive: Some(true),
            watch: None,
            ignore: Some(vec!["$HOME".into(), "$HOME/Downloads".into()]),
            hidden_files: None,
            apply: None,
        };
        let opt2 = Options {
            recursive: Some(false),
            watch: Some(false),
            ignore: Some(vec!["$HOME/Documents".into()]),
            hidden_files: None,
            apply: None,
        };
        let opt3 = Options {
            recursive: Some(true),
            watch: Some(true),
            ignore: Some(vec!["$HOME/Pictures".into()]),
            hidden_files: Some(true),
            apply: Some(Apply::Select(vec![0, 2])),
        };
        let expected = Options {
            recursive: Some(true),
            watch: Some(true),
            ignore: Some({
                let mut ignore1 = opt1.clone().ignore.unwrap();
                let ignore2 = &mut opt2.clone().ignore.unwrap();
                let mut ignore3 = opt3.clone().ignore.unwrap();
                ignore2.append(&mut ignore1);
                ignore3.append(ignore2);
                ignore3
            }),
            hidden_files: Some(true),
            apply: opt3.apply.clone(),
        };
        let one_two = &opt1 + &opt2;
        (&one_two + &opt3 == expected).into_result()
    }
}
