pub trait Capitalize {
	fn capitalize(&self) -> String;
}

impl Capitalize for String {
	fn capitalize(&self) -> Self {
		if self.is_empty() {
			return self.clone();
		}
		let mut c = self.chars();
		c.next().unwrap().to_uppercase().collect::<String>() + c.as_str()
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
