use sqlx::Database;

use crate::db::cache::DbCache;

pub trait TableMetadata<D: Database>: Sized {
	/// Get primary key of this object  
	/// Pk should be i64
	fn pk(&self) -> i64;

	fn build_cache() -> DbCache<Self>;
	/// Get primary key for this object
	fn pk_name() -> &'static str { "id" }
	/// Get table name for this object
	fn tb_name() -> &'static str;
}