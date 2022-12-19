use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};

use dashmap::{DashMap, DashSet};
use dashmap::mapref::one::Ref;
use fxhash::FxBuildHasher;
use tokio::sync::RwLock;

use base::value::{ValueAccess, ValueUpdate};

pub struct DbCache<T> {
	table: DashMap<i64, T, FxBuildHasher>,
	mutation_queue: RwLock<DashSet<i64, FxBuildHasher>>,
	has_handle: AtomicBool,
}

impl<T> DbCache<T> {
	pub fn new(queue_cap: usize) -> Self {
		Self {
			table: DashMap::with_hasher(FxBuildHasher::default()),
			mutation_queue: RwLock::new(DashSet::with_capacity_and_hasher(queue_cap, FxBuildHasher::default())),
			has_handle: AtomicBool::default(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.table.is_empty()
	}

	pub fn should_handle(&self) -> bool {
		if self.has_handle.load(Ordering::Relaxed) {
			false
		} else {
			self.has_handle.store(true, Ordering::Relaxed);
			true
		}
	}

	pub fn get(&self, id: i64) -> Option<Ref<i64, T, FxBuildHasher>> {
		self.table.get(&id)
	}

	pub fn remove(&self, id: i64) -> Option<T> {
		self.table
			.remove(&id)
			.map(|it| it.1)
	}

	pub async fn put(&self, id: i64, value: T) {
		self.table.insert(id, value);
		self.mutation_queue.write().await.insert(id);
	}

	pub async fn modify<F>(&self, id: i64, f: F)
		where F: for<'f> Fn(&'f mut T) -> Pin<Box<dyn Future<Output=()> + 'f>>
	{
		let ent = self.table.get_mut(&id);
		if let Some(mut e) = ent {
			f(&mut e).await;
			self.mutation_queue.write().await.insert(id);
		}
	}

	pub async fn take_queue(&self) -> DashSet<i64, FxBuildHasher> {
		let mut q = self.mutation_queue.write().await;
		let cap = q.capacity();
		// Safety: if nobody taken a queue it won't empty
		mem::replace(&mut *q, DashSet::with_capacity_and_hasher(cap, FxBuildHasher::default()))
	}

	pub fn purge(&self) {
		self.table.clear();
	}
}

impl<T: ValueAccess + ValueUpdate + Clone> DbCache<T> {
	pub async fn merge(&self, key: i64, val: &T, keys: &[&str]) {
		let ent = self.table.get_mut(&key);
		if let Some(mut e) = ent {
			for x in keys {
				if let Some(v) = val.get_value(x) {
					e.set_value(x, v.owned());
				}
			}
		} else {
			self.table.insert(key, val.clone());
		}
	}
}

#[cfg(test)]
mod test {
	use sqlx::Sqlite;

	use crate::db::cache::DbCache;

	fn check_sync_send<T: Sync + Send>() {}

	#[test]
	fn check_cache_sync() {
		check_sync_send::<DbCache<Sqlite>>();
	}
}