mod de;

use crate::config::filters::AsFilter;
use mime::FromStrError;
use std::{convert::TryFrom, ops::Deref, path::Path, str::FromStr};

#[derive(Clone, Debug)]
pub struct Mime(mime::Mime);

impl Deref for Mime {
	type Target = mime::Mime;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl FromStr for Mime {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Mime(mime::Mime::from_str(s)?))
	}
}

#[derive(Clone, Debug)]
pub struct MimeWrapper(Vec<Mime>);

impl Deref for MimeWrapper {
	type Target = Vec<Mime>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl From<Mime> for MimeWrapper {
	fn from(mime: Mime) -> Self {
		MimeWrapper(vec![mime])
	}
}

impl TryFrom<Vec<&str>> for MimeWrapper {
	type Error = FromStrError;

	fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
		let mut vec = Vec::with_capacity(value.len());
		for str in value {
			match Mime::from_str(str) {
				Ok(mime) => vec.push(mime),
				Err(e) => return Err(e),
			}
		}
		Ok(MimeWrapper::new(vec))
	}
}

impl AsFilter for MimeWrapper {
	fn matches<T: AsRef<Path>>(&self, path: &T) -> bool {
		let guess = mime_guess::from_path(path.as_ref()).first_or_octet_stream();
		self.iter().any(|mime| match (mime.type_(), mime.subtype()) {
			(mime::STAR, subtype) => subtype == guess.subtype(),
			(ty, mime::STAR) => ty == guess.type_(),
			(ty, subtype) => ty == guess.type_() && subtype == guess.subtype(),
		})
	}
}

impl MimeWrapper {
	pub fn new(vec: Vec<Mime>) -> Self {
		Self(vec)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_match() {
		let types = MimeWrapper::try_from(vec!["image/*", "audio/*"]).unwrap();
		let img = "test.jpg";
		let audio = "test.ogg";
		assert!(types.matches(&img));
		assert!(types.matches(&audio));
	}
}
