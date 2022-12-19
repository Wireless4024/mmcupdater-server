use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;

use axum::Extension;
use dashmap::DashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::{Database, migrate, Pool, Sqlite};
use tokio::fs::{File, metadata};
use tracing::debug;

use base::value::ValueAccess;
pub use repository::Repository;
pub use table_meta::TableMetadata;

use crate::db::cache::DbCache;

mod repository;
mod table_meta;
pub(crate) mod cache;

pub struct DbWrapper<D: Database> {
	pool: Arc<Pool<D>>,
	cache: Arc<DashMap<&'static str, Arc<dyn Any + Send + Sync>>>,
}

impl<D: Database> Deref for DbWrapper<D> {
	type Target = Arc<Pool<D>>;

	fn deref(&self) -> &Self::Target {
		&self.pool
	}
}

impl<D: Database> Clone for DbWrapper<D> {
	fn clone(&self) -> Self {
		Self { pool: Arc::clone(&self.pool), cache: Arc::clone(&self.cache) }
	}
}

impl<D: Database> DbWrapper<D> {
	pub async fn close(&self) {
		self.pool.close().await
	}

	pub fn repo<T>(&self) -> Repository<D, T> where T: TableMetadata<D> + Serialize + DeserializeOwned + Send + Sync + 'static {
		Repository::new(self)
	}

	pub fn get_cache_for<T: TableMetadata<D> + Send + Sync + 'static>(&self) -> Arc<DbCache<T>> {
		self.cache
			.entry(T::tb_name())
			.or_insert_with(|| Arc::new(T::build_cache()));
		let val = Arc::clone(self.cache.get(T::tb_name())
			.as_ref()
			.unwrap());
		let v = val.downcast::<DbCache<T>>();
		v.unwrap()
	}
}

impl DbWrapper<Sqlite> {
	pub fn repo_with_cache<T>(&self) -> Repository<Sqlite, T>
		where T: TableMetadata<Sqlite>
		+ Serialize
		+ DeserializeOwned
		+ Send
		+ Sync
		+ Clone
		+ Unpin
		+ ValueAccess
		+ 'static {
		Repository::new_with_cache(self)
	}
}

pub type DB = Extension<DbWrapper<Sqlite>>;

pub async fn init() -> anyhow::Result<DbWrapper<Sqlite>> {
	if metadata("db.sqlite").await.is_err() {
		File::create("db.sqlite").await?;
	}
	let pool = Pool::<Sqlite>::connect("sqlite:db.sqlite").await?;
	let migrations = migrate!();
	debug!("running migrations");
	migrations.run(&pool).await?;
	let db = DbWrapper { pool: Arc::new(pool), cache: Default::default() };
	Ok(db)
}