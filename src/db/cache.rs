use std::collections::BTreeMap;
use std::future::Future;
use std::hash::Hash;
use std::mem;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};

use dashmap::{DashMap, DashSet};
use dashmap::mapref::one::Ref;
use fxhash::FxBuildHasher;
use tokio::sync::RwLock;

use base::value::{RustPrimitiveValue, ValueAccess, ValueUpdate};

pub struct DbCache<T> {
	pk_table: DashMap<i64, T, FxBuildHasher>,
	dyn_table: DashMap<String, DashMap<RustPrimitiveValue, i64, FxBuildHasher>, FxBuildHasher>,
	mutation_queue: RwLock<DashSet<i64, FxBuildHasher>>,
	/// Pair<Timestamp, id>
	purge_queue: RwLock<BTreeMap<u64, i64>>,
	has_handle: AtomicBool,
}

#[inline]
fn new_map<K: Eq + Hash, V>() -> DashMap<K, V, FxBuildHasher> {
	DashMap::with_hasher(FxBuildHasher::default())
}

impl<T> DbCache<T> {
	pub fn new(queue_cap: usize) -> Self {
		Self {
			pk_table: new_map(),
			dyn_table: new_map(),
			mutation_queue: RwLock::new(DashSet::with_capacity_and_hasher(queue_cap, FxBuildHasher::default())),
			purge_queue: RwLock::new(BTreeMap::new()),
			has_handle: AtomicBool::default(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.pk_table.is_empty()
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
		self.pk_table.get(&id)
	}

	pub fn get_by(&self, filter: RustPrimitiveValue, k: &str) -> Option<Ref<i64, T, FxBuildHasher>> {
		let a = self.dyn_table.get(k)?;
		self.get(a.get(&filter).map(|it| *it.value())?)
	}

	pub fn remove(&self, id: i64) -> Option<T> {
		self.pk_table
			.remove(&id)
			.map(|it| it.1)
	}

	pub async fn put(&self, id: i64, value: T) {
		self.pk_table.insert(id, value);
		self.mutation_queue.write().await.insert(id);
	}

	pub async fn put_by(&self, id: i64, field: &str, value: T) -> Option<()>
		where T: ValueAccess {
		let k = value.get_value(field)?.owned();
		self.put_by_raw(id, field, k, value).await
	}

	pub async fn put_by_raw(&self, id: i64, field: &str, k: RustPrimitiveValue, value: T) -> Option<()> {
		self.pk_table.insert(id, value);
		self.dyn_table
			.entry(field.to_string())
			.or_insert_with(new_map)
			.insert(k, id);
		self.mutation_queue.write().await.insert(id);
		Some(())
	}

	pub async fn modify<F>(&self, id: i64, f: F)
		where F: for<'f> Fn(&'f mut T) -> Pin<Box<dyn Future<Output=()> + 'f>>
	{
		let ent = self.pk_table.get_mut(&id);
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
		self.pk_table.clear();
	}
}

impl<T: ValueAccess + ValueUpdate + Clone> DbCache<T> {
	pub async fn merge(&self, key: i64, val: &T, keys: &[&str]) {
		let ent = self.pk_table.get_mut(&key);
		if let Some(mut e) = ent {
			for x in keys {
				if let Some(v) = val.get_value(x) {
					e.set_value(x, v.owned());
				}
			}
		} else {
			self.pk_table.insert(key, val.clone());
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