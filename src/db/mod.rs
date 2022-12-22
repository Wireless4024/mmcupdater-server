use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;

use axum::Extension;
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Database, Executor, migrate, Pool, Sqlite};
use tokio::fs::{File, metadata};
use tracing::debug;

use base::value::ValueAccess;
pub use repository::Repository;
pub use table_meta::TableMetadata;

use crate::db::cache::DbCache;

mod repository;
mod table_meta;
pub(crate) mod cache;

pub struct DbWrapper<D: Database, E> where for<'a> &'a E: Executor<'a, Database=D> {
	pool: Arc<E>,
	cache: Arc<DashMap<&'static str, Arc<dyn Any + Send + Sync>>>,
}

impl<D: Database, E> Deref for DbWrapper<D, E> where for<'a> &'a E: Executor<'a, Database=D> {
	type Target = Arc<E>;

	fn deref(&self) -> &Self::Target {
		&self.pool
	}
}

impl<D: Database> Clone for DbWrapper<D, Pool<D>> where for<'a> &'a Pool<D>: Executor<'a, Database=D> {
	fn clone(&self) -> Self {
		Self { pool: Arc::clone(&self.pool), cache: Arc::clone(&self.cache) }
	}
}

impl<D: Database> DbWrapper<D, Pool<D>> where for<'a> &'a Pool<D>: Executor<'a, Database=D> {
	pub async fn close(&self) {
		Pool::<D>::close(self.executor()).await
	}

	pub fn executor(&self) -> &Pool<D> {
		&self.pool
	}

	pub fn repo<T>(&self) -> Repository<D, Pool<D>, T> where T: TableMetadata<D> + Serialize + DeserializeOwned + Send + Sync + 'static {
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

impl DbWrapper<Sqlite, Pool<Sqlite>> {
	pub fn repo_with_cache<T>(&self) -> Repository<Sqlite, Pool<Sqlite>, T>
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

pub type DB = Extension<DbWrapper<Sqlite, Pool<Sqlite>>>;

pub async fn init() -> anyhow::Result<DbWrapper<Sqlite, Pool<Sqlite>>> {
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