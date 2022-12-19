pub trait BoolExt {
	fn maybe_true(&self) -> bool;
}

impl BoolExt for str {
	fn maybe_true(&self) -> bool {
		self.starts_with(['1', 't', 'y'])
	}
}