use std::error::Error;
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
pub struct HttpResult<T: Serialize> {
	success: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	message: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	err_cause: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	result: Option<T>,
}

pub type ResponseResult<T> = std::result::Result<Json<HttpResult<T>>, ErrorWrapper>;

impl<T: Serialize> HttpResult<T> {
	pub fn success(data: T) -> ResponseResult<T> {
		Ok(Json(Self {
			success: true,
			message: None,
			err_cause: None,
			result: Some(data),
		}))
	}

	pub fn err(message: impl Into<String>) -> ResponseResult<T> {
		Ok(Json(Self {
			success: false,
			message: Some(message.into()),
			err_cause: None,
			result: None,
		}))
	}

	pub fn err_with_cause(message: String, cause: String) -> ResponseResult<T> {
		Ok(Json(Self {
			success: false,
			message: Some(message),
			err_cause: Some(cause),
			result: None,
		}))
	}
}

#[derive(Error, Debug)]
pub enum ErrorWrapper {
	#[error("No such element")]
	NotFound,
	#[error("IO Error")]
	IO(#[from] io::Error),
	#[error("HTTP Error")]
	Reqwest(#[from] reqwest::Error),
	#[error("Hyper")]
	Hyper(#[from] hyper::Error),
	#[error("Any how")]
	Anyhow(#[from] anyhow::Error),
	#[error("Unknown Error")]
	Other(#[from] Box<dyn Error + Send + Sync>),
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
		}
	}
}

macro_rules! write_err {
    ($writer:expr, $err:expr) => {
		write!($writer, "{:?}", $err).unwrap();
    };
}

impl IntoResponse for ErrorWrapper {
	fn into_response(self) -> Response {
		if let Self::NotFound = &self {
			return (StatusCode::NOT_FOUND, String::from("Not found")).into_response();
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
				ErrorWrapper::Anyhow(err) => {
					write_err!(writer, err);
				}
				ErrorWrapper::NotFound => {
					error_message.push_str("Not found");
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