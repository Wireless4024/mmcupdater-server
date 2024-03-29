use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

pub enum RustPrimitiveValueRef<'a> {
	Integer(Integer),
	Float(Float),
	String(&'a str),
	Bytes(&'a [u8]),
}

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum RustPrimitiveValue {
	Integer(Integer),
	Float(Float),
	String(String),
	Bytes(Vec<u8>),
}

impl RustPrimitiveValueRef<'_> {
	pub fn owned(&self) -> RustPrimitiveValue {
		match self {
			RustPrimitiveValueRef::Integer(i) => {
				RustPrimitiveValue::Integer(i.clone())
			}
			RustPrimitiveValueRef::Float(f) => {
				RustPrimitiveValue::Float(f.clone())
			}
			RustPrimitiveValueRef::String(s) => {
				RustPrimitiveValue::String(String::from(*s))
			}
			RustPrimitiveValueRef::Bytes(b) => {
				RustPrimitiveValue::Bytes(b.to_vec())
			}
		}
	}
}

impl From<i128> for RustPrimitiveValue {
	fn from(value: i128) -> Self {
		Self::Integer(Integer::I128(value))
	}
}

impl From<f64> for RustPrimitiveValue {
	fn from(value: f64) -> Self {
		Self::Float(Float::F64(value))
	}
}

#[derive(Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum Integer {
	I8(i8),
	I16(i16),
	I32(i32),
	I64(i64),
	I128(i128),
	ISize(isize),
	U8(u8),
	U16(u16),
	U32(u32),
	U64(u64),
	U128(u128),
	USize(usize),
}

#[derive(Clone, Debug)]
pub enum Float {
	F32(f32),
	F64(f64),
}

// impl below is lossy implementation to make derive work only
impl Hash for Float {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self {
			Float::F32(f) => { state.write(&f.to_ne_bytes()) }
			Float::F64(f) => { state.write(&f.to_ne_bytes()) }
		}
	}
}

impl PartialEq for Float {
	fn eq(&self, other: &Self) -> bool {
		match self {
			Float::F32(f) => { f.eq(&(other.as_f64() as f32)) }
			Float::F64(f) => { f.eq(&other.as_f64()) }
		}
	}
}

impl PartialOrd for Float {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		match self {
			Float::F32(f) => { f.partial_cmp(&(other.as_f64() as f32)) }
			Float::F64(f) => { f.partial_cmp(&other.as_f64()) }
		}
	}
}

impl Eq for Float {}

impl Ord for Float {
	fn cmp(&self, other: &Self) -> Ordering {
		match self {
			Float::F32(f) => { f.total_cmp(&(other.as_f64() as f32)) }
			Float::F64(f) => { f.total_cmp(&other.as_f64()) }
		}
	}
}

impl Debug for RustPrimitiveValueRef<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			RustPrimitiveValueRef::Integer(i) => {
				match i {
					Integer::I8(i) => { Debug::fmt(i, f) }
					Integer::I16(i) => { Debug::fmt(i, f) }
					Integer::I32(i) => { Debug::fmt(i, f) }
					Integer::I64(i) => { Debug::fmt(i, f) }
					Integer::I128(i) => { Debug::fmt(i, f) }
					Integer::ISize(i) => { Debug::fmt(i, f) }
					Integer::U8(i) => { Debug::fmt(i, f) }
					Integer::U16(i) => { Debug::fmt(i, f) }
					Integer::U32(i) => { Debug::fmt(i, f) }
					Integer::U64(i) => { Debug::fmt(i, f) }
					Integer::U128(i) => { Debug::fmt(i, f) }
					Integer::USize(i) => { Debug::fmt(i, f) }
				}
			}
			RustPrimitiveValueRef::Float(i) => {
				Debug::fmt(&i.as_f64(), f)
			}
			RustPrimitiveValueRef::String(s) => {
				Debug::fmt(s, f)
			}
			RustPrimitiveValueRef::Bytes(b) => {
				Debug::fmt(b, f)
			}
		}
	}
}

macro_rules! cast_int {
    ($it:ident) => {
	    match $it {
			Integer::I8(i) => { *i as _ }
			Integer::I16(i) => { *i as _ }
			Integer::I32(i) => { *i as _ }
			Integer::I64(i) => { *i as _ }
			Integer::I128(i) => { *i as _ }
			Integer::ISize(i) => { *i as _ }
			Integer::U8(i) => { *i as _ }
			Integer::U16(i) => { *i as _ }
			Integer::U32(i) => { *i as _ }
			Integer::U64(i) => { *i as _ }
			Integer::U128(i) => { *i as _ }
			Integer::USize(i) => { *i as _ }
		}
    };
}

impl Integer {
	#[inline]
	pub fn as_i128(&self) -> i128 {
		cast_int!(self)
	}

	#[inline]
	pub fn as_u128(&self) -> u128 {
		cast_int!(self)
	}
}

impl Float {
	pub fn as_f64(&self) -> f64 {
		match self {
			Float::F32(f) => { *f as _ }
			Float::F64(f) => { *f }
		}
	}
}

pub trait ValueAccess {
	fn get_value(&self, key: &str) -> Option<RustPrimitiveValueRef>;
}

pub enum ValueUpdateResult {
	TypeMismatch(RustPrimitiveValue),
	Invalid,
	Success,
}

pub trait ValueUpdate {
	fn set_value(&mut self, key: &str, value: RustPrimitiveValue) -> ValueUpdateResult;
	fn merge_from<T: ValueAccess + ?Sized>(&mut self, from: &T, keys: &[&str]) {
		for x in keys {
			if let Some(v) = from.get_value(x) {
				self.set_value(x, v.owned());
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::value::Float::{F32, F64};

	#[test]
	fn test() {
		assert_eq!(F32(1.0), F32(1.0));
		assert_eq!(F32(2.0), F64(2.0));
	}
}