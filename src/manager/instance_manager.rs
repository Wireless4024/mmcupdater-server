use std::sync::Arc;

use hashbrown::HashMap;
use tokio::sync::RwLock;

use crate::instance::mc_instance::McInstance;

type Instance = Arc<RwLock<McInstance>>;

pub struct InstanceManager {
	pub instances: HashMap<String, Instance>,
}

impl InstanceManager {
	pub fn names(&self) -> Vec<&str> {
		self.instances.keys().map(|it| it.as_str()).collect()
	}

	pub fn find(&self, name: &str) -> Option<Instance> {
		self.instances.get(name).map(Arc::clone)
	}
}