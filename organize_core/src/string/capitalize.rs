pub trait Capitalize {
	fn capitalize(self) -> String;
}

impl Capitalize for String {
	fn capitalize(self) -> Self {
		let mut chars = self.chars();
		if let Some(char) = chars.next() {
			char.to_uppercase().to_string() + chars.as_str()
		} else {
			self
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn capitalize_word() {
		let tested = String::from("house");
		let expected = String::from("House");
		assert_eq!(tested.capitalize(), expected)
	}
	#[test]
	fn capitalize_single_char() {
		let tested = String::from("h");
		let expected = String::from("H");
		assert_eq!(tested.capitalize(), expected)
	}
}
