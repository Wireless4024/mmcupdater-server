use std::borrow::Cow;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serde::{Deserializer, Serialize};
use serde::de::{DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use sqlx::{Column, Database, Decode, Executor, Row, Sqlite, Statement, TypeInfo, ValueRef};
use sqlx::query::Query;
use sqlx::sqlite::{SqliteArguments, SqliteColumn, SqliteRow, SqliteValueRef};
use tokio::time::sleep;

use base::{RustPrimitiveValueRef, ValueAccess};

use crate::db::cache::DbCache;
use crate::db::DbWrapper;
use crate::db::table_meta::TableMetadata;
use crate::util::errors::ErrorWrapper;
use crate::util::modification::ModificationTracker;
use crate::util::serde::{DeserializeError, get_field_names};

pub struct Repository<D: Database, T: Serialize + DeserializeOwned> {
	con: DbWrapper<D>,
	cache: Option<Arc<DbCache<T>>>,
}

impl<D: Database, T: Serialize + DeserializeOwned> Clone for Repository<D, T> {
	fn clone(&self) -> Self {
		Self {
			con: DbWrapper::clone(&self.con),
			cache: self.cache.as_ref().map(Arc::clone),
		}
	}
}

impl<D: Database, T: Serialize + DeserializeOwned> Repository<D, T> {
	pub fn new(con: &DbWrapper<D>) -> Self {
		Self {
			con: DbWrapper::clone(con),
			cache: None,
		}
	}
}

impl<T> Repository<Sqlite, T>
	where for<'a> T: Serialize
	+ DeserializeOwned
	+ TableMetadata<Sqlite>
	+ Send
	+ Sync
	+ Unpin
	+ 'static
	+ Clone
	+ ValueAccess
{
	pub fn new_with_cache(con: &DbWrapper<Sqlite>) -> Self {
		let cache: Arc<DbCache<T>> = con.get_cache_for();
		let repo = Self {
			con: DbWrapper::clone(con),
			cache: Some(Arc::clone(&cache)),
		};
		if cache.should_handle() {
			let repo = repo.clone();
			tokio::spawn(async move {
				loop {
					sleep(Duration::from_secs(10)).await;
					let queue = cache.take_queue().await;
					if !queue.is_empty() {
						for x in queue {
							if let Some(it) = cache.get(x) {
								repo.save(&it).await.ok();
							}
						}
					}
				}
			});
		}
		repo
	}

	pub async fn create(&self) -> Result<T, ErrorWrapper> {
		let stmt = self.con.prepare("INSERT INTO \"{}\" DEFAULT VALUES RETURNING *").await?;
		let row = stmt.query().fetch_one(&**self.con).await?;
		Ok(T::deserialize(RowDeserializer(row, 0))?)
	}

	pub async fn get(&self, pk: i64) -> Option<T> {
		if let Some(cache) = &self.cache {
			if let Some(item) = cache.get(pk) {
				return Some(item.value().clone());
			}
		}

		let query = format!("SELECT * FROM \"{}\" WHERE {} = $1", T::tb_name(), T::pk_name());
		let stmt = self.con.prepare(&query).await.ok()?;
		let query = stmt.query();
		let row = query.bind(pk).fetch_one(&**self.con).await.ok()?;
		let value = T::deserialize(RowDeserializer(row, 0)).expect("deserialize");

		if let Some(cache) = &self.cache {
			cache.put(pk, value.clone()).await;
		}
		Some(value)
	}

	pub async fn get_by<'a, 'b: 'a>(&'a self, keys: &'static [&'static str], val: &'b T) -> Option<T> {
		let mut fields = keys.iter();

		let mut query = String::with_capacity(64);
		query.push_str("SELECT * FROM ");
		query.push_str(T::tb_name());
		query.push_str(" WHERE ");
		query.push_str(fields.next().unwrap());
		query.push_str("=$1");
		let mut i = 2;
		use std::fmt::Write;
		for f in fields {
			query.push(',');
			query.push_str(f);
			query.push_str("=$");
			let _ = write!(query, "{i}");
			i += 1;
		}
		let stmt = self.con.prepare(&query).await.ok()?;
		let mut query = stmt.query();
		query = Self::bind(query, keys, val);
		let row = query.fetch_one(&**self.con).await.ok()?;
		let value = T::deserialize(RowDeserializer(row, 0)).expect("deserialize");
		if let Some(cache) = &self.cache {
			cache.put(value.pk(), value.clone()).await;
		}
		Some(value)
	}

	pub async fn update(&self, val: &T) -> Result<()> {
		if let Some(cache) = &self.cache {
			if val.pk() != 0 {
				cache.put(val.pk(), val.clone()).await;
			} else {
				self.perform_insert(val).await?;
			}
			Ok(())
		} else {
			self.save(val).await
		}
	}

	pub async fn save(&self, val: &T) -> Result<()> {
		let mut keys = get_field_names::<T>().to_vec();
		let pk_name = T::pk_name();
		let pk = keys.remove(keys.iter().position(|it| *it == pk_name).unwrap());
		let mut fields = keys.iter();

		let mut query = String::with_capacity(64);
		query.push_str("UPDATE ");
		query.push_str(T::tb_name());
		query.push_str(" SET ");
		query.push_str(fields.next().unwrap());
		query.push_str("=$1");
		let mut i = 2;
		use std::fmt::Write;
		for f in fields {
			query.push(',');
			query.push_str(f);
			query.push_str("=$");
			let _ = write!(query, "{i}");
			i += 1;
		}
		query.push_str(" WHERE ");
		query.push_str(T::pk_name());
		query.push_str("=$");
		let _ = write!(query, "{i}");
		keys.push(pk);
		let stmt = self.con.prepare(&query).await?;
		let mut query = stmt.query();
		query = Self::bind(query, &keys, val);
		let _ = query.execute(&**self.con).await?;
		Ok(())
	}

	async fn perform_insert(&self, val: &T) -> Result<()> {
		let mut keys = get_field_names::<T>().to_vec();
		let pk_name = T::pk_name();
		keys.remove(keys.iter().position(|it| *it == pk_name).unwrap());
		let mut fields = keys.iter();

		let mut query = String::with_capacity(64);
		query.push_str("INSERT INTO ");
		query.push_str(T::tb_name());
		query.push('(');
		query.push_str(fields.next().unwrap());
		use std::fmt::Write;
		for f in fields {
			query.push(',');
			query.push_str(f);
		}
		query.push_str(")VALUES($1");
		let mut i = 2;
		for _ in 1..keys.len() {
			let _ = write!(query, ",${i}");
			i += 1;
		}
		query.push(')');
		let stmt = self.con.prepare(&query).await?;
		let mut query = stmt.query();
		query = Self::bind(query, &keys, val);
		let _ = query.execute(&**self.con).await?;
		Ok(())
	}

	fn bind<'q, 'b: 'q>(mut q: Query<'q, Sqlite, SqliteArguments<'q>>, keys: &[&str], val: &'b T) -> Query<'q, Sqlite, SqliteArguments<'q>> {
		for x in keys {
			match val.get_value(x).unwrap() {
				RustPrimitiveValueRef::Integer(i) => {
					q = q.bind(i.as_i128() as i64);
				}
				RustPrimitiveValueRef::Float(f) => {
					q = q.bind(f.as_f64());
				}
				RustPrimitiveValueRef::String(s) => {
					q = q.bind(s);
				}
				RustPrimitiveValueRef::Bytes(b) => {
					q = q.bind(b);
				}
			}
		}
		q
	}

	pub async fn delete(&self, pk: i64) -> Result<bool, ErrorWrapper> {
		if let Some(cache) = &self.cache {
			cache.remove(pk);
		}
		let query = format!("DELETE FROM \"{}\" WHERE {} = $1", T::tb_name(), T::pk_name());
		let stmt = self.con.prepare(&query).await?;
		let query = stmt.query();
		let q = query.bind(pk);
		let res = q.execute(&**self.con).await?;
		Ok(res.rows_affected() != 0)
	}
}

impl<'de, T> Repository<Sqlite, T>
	where T: Serialize
	+ DeserializeOwned
	+ TableMetadata<Sqlite>
	+ Send
	+ Sync
	+ Unpin
	+ 'static
	+ Clone
	+ ValueAccess
	+ Deref<Target=ModificationTracker> {
	pub async fn update_minimal(&self, value: &T) -> Result<(), ErrorWrapper> {
		if self.cache.is_some() {
			self.update_minimal_owned(value.clone()).await
		} else {
			Self::_update(self, value).await
		}
	}

	async fn _update(&self, value: &T) -> Result<(), ErrorWrapper> {
		let mut keys = value.take_modifications();
		let mut fields = keys.iter();

		let mut query = String::with_capacity(64);
		query.push_str("UPDATE ");
		query.push_str(T::tb_name());
		query.push_str(" SET ");
		query.push_str(fields.next().unwrap());
		query.push_str("=$1");
		let mut i = 2;
		use std::fmt::Write;
		for f in fields {
			query.push(',');
			query.push_str(f);
			query.push_str("=$");
			let _ = write!(query, "{i}");
			i += 1;
		}
		query.push_str(" WHERE ");
		query.push_str(T::pk_name());
		query.push_str("=$");
		let _ = write!(query, "{i}");
		keys.push(Cow::Borrowed(T::pk_name()));
		let k = keys.iter().map(|it| it.as_ref()).collect::<Vec<_>>();
		let stmt = self.con.prepare(&query).await?;
		let mut query = stmt.query();
		query = Self::bind(query, &k, value);
		let _ = query.execute(&**self.con).await?;
		Ok(())
	}

	pub async fn update_minimal_owned(&self, value: T) -> Result<(), ErrorWrapper> {
		match &self.cache {
			None => {
				Self::_update(self, &value).await
			}
			Some(cache) => {
				cache.put(value.pk(), value).await;
				Ok(())
			}
		}
	}
}

struct RowDeserializer<D: Database, R: Row<Database=D>>(R, usize);

impl<D: Database, R: Row<Database=D>> Deref for RowDeserializer<D, R> {
	type Target = R;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'de> MapAccess<'de> for RowDeserializer<Sqlite, SqliteRow> {
	type Error = DeserializeError;

	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> where K: DeserializeSeed<'de> {
		if self.1 >= (self.0.len() as _) {
			self.1 = usize::MAX;
			return Ok(None);
		}
		let raw = self.0.column(self.1);
		seed.deserialize(ColumnDeserializer(raw)).map(Some)
	}

	fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error> where V: DeserializeSeed<'de> {
		let idx = self.1;
		self.1 += 1;
		let raw = self.try_get_raw(idx)?;
		seed.deserialize(ColDeserializer(raw))
	}
}

impl<'de> SeqAccess<'de> for RowDeserializer<Sqlite, SqliteRow> {
	type Error = DeserializeError;

	fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
		if self.1 >= self.0.len() {
			return Ok(None);
		}
		let idx = self.1;
		self.1 += 1;
		let raw = self.try_get_raw(idx)?;
		seed.deserialize(ColDeserializer(raw)).map(Some)
	}
}

impl<'de> Deserializer<'de> for RowDeserializer<Sqlite, SqliteRow> {
	type Error = DeserializeError;

	fn deserialize_any<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_bool<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_i8<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_i16<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_i32<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_i64<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_u8<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_u16<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_u32<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_u64<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_f32<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_f64<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_char<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_str<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_string<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_bytes<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_byte_buf<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_option<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_unit<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_unit_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_newtype_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		visitor.visit_seq(self)
	}

	fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		visitor.visit_seq(self)
	}

	fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		visitor.visit_seq(self)
	}

	fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		visitor.visit_map(self)
	}

	fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		visitor.visit_map(self)
	}

	fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_identifier<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}

	fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		unimplemented!()
	}
}

struct ColDeserializer<'de>(SqliteValueRef<'de>);

struct ColumnDeserializer<'de>(&'de SqliteColumn);

macro_rules! decode {
    ($self:ident,$iden:ident,$vis:ident) => {
	    {
		    if $self.0.is_null() { 
				return $vis.visit_none();
			}
		    $iden::decode($self.0).map_err(|it| DeserializeError(it.to_string()))?
	    }
    };
}
macro_rules! try_unwrap {
    ($self:ident,$iden:ident,$vis:ident) => {
	    {
		    if $self.0.is_null() { 
				return $vis.visit_none();
			}
		    $iden::decode($self.0).map_err(|it| DeserializeError(it.to_string()))?
	    }
    };
}

impl<'a, 'de> Deserializer<'de> for ColDeserializer<'a> {
	type Error = DeserializeError;

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let typ = self.0.type_info();
		match typ.name() {
			"NULL" => {
				visitor.visit_none()
			}
			"TEXT" => {
				self.deserialize_string(visitor)
			}
			"REAL" => {
				self.deserialize_f64(visitor)
			}
			"BLOB" => {
				self.deserialize_bytes(visitor)
			}
			"INTEGER" => {
				self.deserialize_i64(visitor)
			}
			"NUMERIC" => {
				self.deserialize_f64(visitor)
			}
			"BOOLEAN" => {
				self.deserialize_bool(visitor)
			}
			"DATE" => {
				// TODO: verify
				self.deserialize_i64(visitor)
			}
			"TIME" => {
				self.deserialize_i64(visitor)
			}
			"Datetime" => {
				self.deserialize_i64(visitor)
			}
			_ => {
				Err(DeserializeError("Unsupported".to_string()))
			}
		}
	}

	fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, bool, visitor);
		visitor.visit_bool(s)
	}

	fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, i64, visitor);
		visitor.visit_i8(s as _)
	}

	fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, i64, visitor);
		visitor.visit_i16(s as _)
	}

	fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, i64, visitor);
		visitor.visit_i32(s as _)
	}

	fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, i64, visitor);
		visitor.visit_i64(s)
	}

	fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, i64, visitor);
		visitor.visit_u8(s as _)
	}

	fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, i64, visitor);
		visitor.visit_u16(s as _)
	}

	fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, i64, visitor);
		visitor.visit_u32(s as _)
	}

	fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, i64, visitor);
		visitor.visit_u64(s as _)
	}

	fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, f64, visitor);
		visitor.visit_f32(s as f32)
	}

	fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, f64, visitor);
		visitor.visit_f64(s)
	}

	fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s = decode!(self, String, visitor);
		match s.chars().next() {
			Some(ch) => {
				visitor.visit_char(ch)
			}
			None => {
				visitor.visit_none()
			}
		}
	}

	fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		self.deserialize_string(visitor)
	}

	fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s: String = String::decode(self.0).map_err(|it| DeserializeError(it.to_string()))?;
		visitor.visit_string(s)
	}

	fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		self.deserialize_byte_buf(visitor)
	}

	fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		let s: Vec<u8> = Vec::<u8>::decode(self.0).map_err(|it| DeserializeError(it.to_string()))?;
		visitor.visit_byte_buf(s)
	}

	fn deserialize_option<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_unit<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_unit_struct<V>(self, name: &'static str, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_newtype_struct<V>(self, name: &'static str, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_seq<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_tuple<V>(self, len: usize, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_map<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_identifier<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}
}

impl<'a, 'de> Deserializer<'de> for ColumnDeserializer<'a> {
	type Error = DeserializeError;

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		self.deserialize_str(visitor)
	}

	fn deserialize_bool<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_i8<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_i16<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_i32<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_i64<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_u8<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_u16<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_u32<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_u64<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_f32<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_f64<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_char<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		visitor.visit_str(self.0.name())
	}

	fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		self.deserialize_str(visitor)
	}

	fn deserialize_bytes<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_byte_buf<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_option<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_unit<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_unit_struct<V>(self, name: &'static str, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_newtype_struct<V>(self, name: &'static str, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_seq<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_tuple<V>(self, len: usize, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_tuple_struct<V>(self, _: &'static str, _: usize, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_map<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_struct<V>(self, _: &'static str, _: &'static [&'static str], _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_enum<V>(self, _: &'static str, _: &'static [&'static str], _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		visitor.visit_str(self.0.name())
	}

	fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}
}