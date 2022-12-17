use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};

use anyhow::anyhow;
use hashbrown::HashSet;
use serde::{Deserialize, Deserializer, Serializer};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use sqlx::Error;
use thiserror::Error;

use crate::util::errors::ErrorWrapper;

#[derive(Error, Debug)]
pub struct DeserializeError(pub String);

impl Display for DeserializeError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		std::fmt::Debug::fmt(self, f)
	}
}

impl serde::de::Error for DeserializeError {
	fn custom<T>(msg: T) -> Self where T: Display {
		DeserializeError(format!("{msg}"))
	}
}

impl serde::ser::Error for DeserializeError {
	fn custom<T>(msg: T) -> Self where T: Display {
		DeserializeError(format!("{msg}"))
	}
}

impl From<sqlx::Error> for DeserializeError {
	fn from(msg: Error) -> Self {
		DeserializeError(format!("{msg}"))
	}
}

impl From<DeserializeError> for ErrorWrapper {
	#[inline]
	fn from(value: DeserializeError) -> Self {
		Self::Anyhow(anyhow!("{value}"))
	}
}

pub fn str_to_set<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
	where
		D: Deserializer<'de>,
{
	let s: Cow<'de, str> = Deserialize::deserialize(deserializer)?;
	let mut set = HashSet::new();
	for x in s.split(',') {
		set.insert(x.to_string());
	}
	Ok(set)
}

pub fn set_to_str<S>(set: &HashSet<String>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
{
	let mut s = String::new();
	let mut iter = set.iter();
	if let Some(x) = iter.next() {
		s.push_str(x);
	}
	for x in iter {
		// split safety
		if x.contains(',') { continue; }
		s.push(',');
		s.push_str(x);
	}
	serializer.serialize_str(&s)
}

pub fn get_field_names<'de, S: Deserialize<'de>>() -> &'static [&'static str] {
	match S::deserialize(GetField { inner: &[] }) {
		Ok(_) => &[],
		Err(e) => e.inner,
	}
}

#[derive(Error, Debug)]
struct GetField {
	inner: &'static [&'static str],
}

impl Display for GetField {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		std::fmt::Debug::fmt(self, f)
	}
}

impl serde::de::Error for GetField {
	fn custom<T>(_: T) -> Self where T: Display {
		Self { inner: &[] }
	}
}

impl<'de> Deserializer<'de> for GetField {
	type Error = GetField;

	fn deserialize_any<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
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

	fn deserialize_str<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_string<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
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

	fn deserialize_unit_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_newtype_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_seq<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_tuple<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_tuple_struct<V>(self, _: &'static str, _: usize, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_map<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_struct<V>(mut self, _: &'static str, fields: &'static [&'static str], _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		self.inner = fields;
		Err(self)
	}

	fn deserialize_enum<V>(self, _: &'static str, _: &'static [&'static str], _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_identifier<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}

	fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
		todo!()
	}
}