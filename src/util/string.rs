pub trait BoolExt {
	fn maybe_true(&self) -> bool;
}

pub trait EmptyExt {
	fn is_empty(&self) -> bool;
}

impl BoolExt for str {
	fn maybe_true(&self) -> bool {
		self.starts_with(['1', 't', 'y'])
	}
}

impl EmptyExt for Option<String> {
	fn is_empty(&self) -> bool {
		self.as_ref().and_then(|it| if it.is_empty() { None } else { Some(()) }).is_none()
	}
}