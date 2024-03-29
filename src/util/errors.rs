use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind;

use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;
use zip::result::ZipError;

use crate::util::safe_writer::SafeWriter;

pub fn reqwest_to_io(err: reqwest::Error) -> io::Error {
	io::Error::new(ErrorKind::Other, err)
}

pub fn zip_to_io(err: ZipError) -> io::Error {
	io::Error::new(ErrorKind::Other, err)
}

pub type Result<T> = std::result::Result<T, ErrorWrapper>;

#[derive(Serialize)]
pub struct ResultBase<T: Serialize, M: Serialize> {
	success: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	message: Option<M>,
	#[serde(skip_serializing_if = "Option::is_none")]
	err_cause: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	result: Option<T>,
}

pub type ResponseResult<T, M = &'static str> = std::result::Result<Json<ResultBase<T, M>>, ErrorWrapper>;

pub mod rest {
	use axum::body::BoxBody;
	use axum::http::{Response, StatusCode};
	use axum::response::IntoResponse;
	use serde::Serialize;

	use crate::util::errors::{ErrorWrapper, ResultBase};

	pub type Resp = Result<Response<BoxBody>, ErrorWrapper>;

	pub fn got<T: Serialize>(data: T) -> Resp {
		Ok(ResultBase::success(data).into_response())
	}

	pub fn no_content() -> Resp {
		Ok(StatusCode::NO_CONTENT.into_response())
	}

	pub fn not_found() -> Resp {
		Ok(StatusCode::NOT_FOUND.into_response())
	}

	pub fn conflict() -> Resp {
		Ok(StatusCode::CONFLICT.into_response())
	}

	pub fn created<T: Serialize>(data: T) -> Resp {
		let mut res = ResultBase::success(data).into_response();
		*res.status_mut() = StatusCode::CREATED;
		Ok(res)
	}

	pub fn updated<T: Serialize>(data: T) -> Resp {
		Ok(ResultBase::success(data).into_response())
	}

	pub fn deleted<T: Serialize>(data: T) -> Resp {
		Ok(ResultBase::success(data).into_response())
	}
}

impl<M: Serialize> ResultBase<String, M> {
	pub const fn err_raw(message: M) -> Json<Self> {
		Json(Self {
			success: false,
			message: Some(message),
			err_cause: None,
			result: None,
		})
	}
}

impl<T: Serialize> ResultBase<T, &'static str> {
	pub const fn success(data: T) -> std::result::Result<Json<Self>, ErrorWrapper> {
		Ok(Json(Self {
			success: true,
			message: None,
			err_cause: None,
			result: Some(data),
		}))
	}

	pub const fn success_raw(result: T) -> Json<Self> {
		Json(Self {
			success: true,
			message: None,
			err_cause: None,
			result: Some(result),
		})
	}
}

impl<T: Serialize, M: Serialize> ResultBase<T, M> {
	pub const fn err(message: M) -> std::result::Result<Json<Self>, ErrorWrapper> {
		Ok(Json(Self {
			success: false,
			message: Some(message),
			err_cause: None,
			result: None,
		}))
	}

	pub const fn err_with_cause(message: M, cause: String) -> std::result::Result<Json<Self>, ErrorWrapper> {
		Ok(Json(Self {
			success: false,
			message: Some(message),
			err_cause: Some(cause),
			result: None,
		}))
	}
}

#[derive(Error, Debug)]
pub enum ErrorWrapper<C: IntoResponse + Debug = &'static str> {
	#[error("No such element")]
	NotFound,
	#[error("IO Error")]
	IO(#[from] io::Error),
	#[error("HTTP Error")]
	Reqwest(#[from] reqwest::Error),
	#[error("Hyper")]
	Hyper(#[from] hyper::Error),
	#[error("Anyhow")]
	Anyhow(#[from] anyhow::Error),
	#[error("Anyhow")]
	Database(#[from] sqlx::Error),
	#[error("Unknown Error")]
	Other(#[from] Box<dyn Error + Send + Sync>),
	#[error("Custom Error")]
	Custom(StatusCode, C),
}

impl<C: IntoResponse + Debug> ErrorWrapper<C> {
	pub fn custom(status: StatusCode, message: C) -> Self {
		Self::Custom(status, message)
	}
}
/*
impl<T> From<Option<T>> for ErrorWrapper{
	fn from(value: Option<T>) -> Self {
		match value {
			None => {}
			Some(_) => {}
		}
	}
}*/

impl From<ErrorWrapper> for io::Error {
	fn from(value: ErrorWrapper) -> Self {
		match value {
			ErrorWrapper::NotFound => { io::Error::new(ErrorKind::NotFound, "Not found") }
			ErrorWrapper::IO(it) => { it }
			ErrorWrapper::Reqwest(err) => {
				io::Error::new(ErrorKind::Other, err)
			}
			ErrorWrapper::Hyper(err) => {
				io::Error::new(ErrorKind::Other, err)
			}
			ErrorWrapper::Other(err) => {
				io::Error::new(ErrorKind::Other, err)
			}
			ErrorWrapper::Anyhow(err) => {
				io::Error::new(ErrorKind::Other, err)
			}
			ErrorWrapper::Database(err) => {
				io::Error::new(ErrorKind::Other, err)
			}
			ErrorWrapper::Custom(_, msg) => {
				io::Error::new(ErrorKind::Other, format!("{msg:?}"))
			}
		}
	}
}

macro_rules! write_err {
    ($writer:expr, $err:expr) => {
		write!($writer, "{:?}", $err).unwrap();
    };
}

impl<C: IntoResponse + Debug> IntoResponse for ErrorWrapper<C> {
	fn into_response(self) -> Response {
		match &self {
			Self::NotFound => {
				return (StatusCode::NOT_FOUND, String::from("Not found")).into_response();
			}
			Self::Custom(..) => {
				if let Self::Custom(code, msg) = self {
					return (code, msg).into_response();
				}
			}
			_ => {}
		}
		#[cfg(debug_assertions)]
			let mut error_message = String::with_capacity(2048);
		#[cfg(not(debug_assertions))]
			let mut error_message = String::with_capacity(256);
		error_message.push_str(r#"{"success":false,"message":""#);
		use std::fmt::Write;
		{
			let mut writer = SafeWriter::new(&mut error_message);
			match self {
				ErrorWrapper::IO(err) => {
					write!(writer, "io::{:?}", err).unwrap();
				}
				ErrorWrapper::Reqwest(err) => {
					write_err!(writer, err);
				}
				ErrorWrapper::Other(err) => {
					write_err!(writer, err);
				}
				ErrorWrapper::Hyper(err) => {
					write_err!(writer, err);
				}
				ErrorWrapper::Database(err) => {
					write_err!(writer, err);
				}
				ErrorWrapper::Anyhow(err) => {
					write_err!(writer, err);
				}
				ErrorWrapper::NotFound | ErrorWrapper::Custom(..) => {
					unreachable!();
				}
			}
		}
		error_message.push_str(r#""}"#);
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			error_message,
		).into_response()
	}
}