use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct User {
	id: i32,
	username: String,
	password: String,
	permissions: HashSet<String>,
}

impl User {
	pub fn has_permission(&self, permission: &str) -> bool {
		self.permissions.contains(permission)
	}
}