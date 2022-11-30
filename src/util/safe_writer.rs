use std::fmt::Write;

// This wrapper will replace `"` with `\"`
pub struct SafeWriter<'a> {
	writer: &'a mut String,
}

impl<'a> SafeWriter<'a> {
	pub fn new(source: &'a mut String) -> Self {
		Self {
			writer: source
		}
	}
}

impl<'a> Write for SafeWriter<'a> {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		self.writer.reserve(s.len());
		let mut cur = 0;
		let mut last = 0;
		let len = s.len();
		let bytes = s.as_bytes();
		fn try_push(target: &mut String, src: &str, from: usize, to: usize) {
			if from != to {
				target.push_str(src.get(from..to).unwrap());
			}
		}
		while cur < len {
			let b = bytes[cur];
			match b {
				b'"' => {
					try_push(self.writer, s, last, cur);
					last = cur + 1;
					self.writer.push_str("\\\"")
				}
				b'\\' => {
					try_push(self.writer, s, last, cur);
					last = cur + 1;
					self.writer.push_str("\\\\")
				}
				b'\n' => {
					try_push(self.writer, s, last, cur);
					last = cur + 1;
					self.writer.push_str("\\n")
				}
				b'\t' => {
					try_push(self.writer, s, last, cur);
					last = cur + 1;
					self.writer.push_str("\\t")
				}
				_ => {}
			};
			cur += 1;
		}
		try_push(self.writer, s, last, len);
		Ok(())
	}

	fn write_char(&mut self, c: char) -> std::fmt::Result {
		match c {
			'"' => self.writer.push_str("\\\""),
			'\\' => self.writer.push_str("\\\\"),
			'\n' => self.writer.push_str("\\\n"),
			'\t' => self.writer.push_str("\\\t"),
			c => self.writer.push(c)
		};
		Ok(())
	}
}