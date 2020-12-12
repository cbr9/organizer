pub trait Capitalize {
	fn capitalize(self) -> String;
}

impl<T: Into<String>> Capitalize for T {
	fn capitalize(self) -> String {
		let str = self.into();
		let mut chars = str.chars();
		if let Some(char) = chars.next() {
			char.to_uppercase().to_string() + chars.as_str()
		} else {
			// it's empty
			str
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn capitalize_word() {
		let tested = "house";
		let expected = "House";
		assert_eq!(tested.capitalize(), expected)
	}
	#[test]
	fn capitalize_single_char() {
		let tested = "h";
		let expected = "H";
		assert_eq!(tested.capitalize(), expected)
	}
	#[test]
	fn capitalize_empty_string() {
		let tested = "";
		let expected = "";
		assert_eq!(tested.capitalize(), expected);
	}
}
