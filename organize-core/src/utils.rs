pub trait DefaultOpt {
	fn default_none() -> Self;
	fn default_some() -> Self;
}

pub trait UnwrapOrDefaultOpt<T: DefaultOpt> {
	fn unwrap_or_default_none(self) -> T;
	fn unwrap_or_default_some(self) -> T;
}

impl<T> UnwrapOrDefaultOpt<T> for Option<T>
where
	T: DefaultOpt,
{
	fn unwrap_or_default_none(self) -> T {
		match self {
			None => T::default_none(),
			Some(obj) => obj,
		}
	}

	fn unwrap_or_default_some(self) -> T {
		match self {
			None => T::default_some(),
			Some(obj) => obj,
		}
	}
}

pub trait Contains<T> {
	fn contains(&self, value: T) -> bool;
}
