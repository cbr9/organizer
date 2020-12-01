mod de;

use crate::config::filters::AsFilter;
use mime::FromStrError;
use std::{ops::Deref, path::Path, str::FromStr};

#[derive(Clone, Debug)]
pub struct Mime(mime::Mime);

#[derive(Clone, Debug)]
pub struct MimeWrapper(Vec<Mime>);

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

impl AsFilter for MimeWrapper {
	fn matches(&self, path: &Path) -> bool {
		let guess = mime_guess::from_path(path).first_or_octet_stream();
		self.0.iter().any(|mime| match (mime.type_(), mime.subtype()) {
			(mime::STAR, subtype) => subtype == guess.subtype(),
			(ty, mime::STAR) => ty == guess.type_(),
			(ty, subtype) => ty == guess.type_() && subtype == guess.subtype(),
		})
	}
}
