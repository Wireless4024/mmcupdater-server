use std::borrow::Cow;
use std::mem;
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct ModificationTracker {
	fields: RwLock<Vec<Cow<'static, str>>>,
}

impl ModificationTracker {
	pub fn log_modify(&self, name: Cow<'static, str>) {
		let mut vec = {
			loop {
				if let Ok(borrow) = self.fields.write() {
					break borrow;
				}
			}
		};
		if !vec.contains(&name) {
			vec.push(name)
		}
	}

	pub fn log_modify_static(&self, name: &'static str) {
		let mut vec = loop {
			if let Ok(borrow) = self.fields.write() {
				break borrow;
			}
		};
		let cow = Cow::Borrowed(name);
		if !vec.contains(&cow) {
			vec.push(cow)
		}
	}

	pub fn take_modifications(&self) -> Vec<Cow<'static, str>> {
		let mut vec = loop {
			if let Ok(borrow) = self.fields.write() {
				break borrow;
			}
		};
		mem::take(&mut *vec)
	}
}

#[macro_export]
macro_rules! mod_field {
    ($struc:ident.$field:ident) => {
	    impl Deref for $struc {
			type Target = ModificationTracker;
	
			fn deref(&self) -> &Self::Target {
				&self.$field
			}
		}
    };
}