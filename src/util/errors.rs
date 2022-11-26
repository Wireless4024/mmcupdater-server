use std::{io, path};
use std::io::ErrorKind;

use thiserror::Error;
use zip::result::ZipError;

pub fn reqwest_to_io(err: reqwest::Error) -> io::Error {
	io::Error::new(ErrorKind::Other, err)
}

pub fn sp_to_io(err: path::StripPrefixError) -> io::Error {
	io::Error::new(ErrorKind::InvalidInput, err)
}

pub fn zip_to_io(err: ZipError) -> io::Error {
	io::Error::new(ErrorKind::Other, err)
}

pub type Result<T> = std::result::Result<T, ErrorWrapper>;

#[derive(Error, Debug)]
pub enum ErrorWrapper {
	#[error("No such element")]
	NotFound,
	#[error("IO Error")]
	IO(#[from] io::Error),
	#[error("HTTP Error")]
	REQWEST(#[from] reqwest::Error),
	#[error("Hyper")]
	HYPER(#[from] hyper::Error),
	#[error("Unknown Error")]
	OTHER(#[from] Box<dyn std::error::Error + Send + Sync>),
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
			ErrorWrapper::REQWEST(err) => {
				io::Error::new(ErrorKind::Other, err)
			}
			ErrorWrapper::HYPER(err) => {
				io::Error::new(ErrorKind::Other, err)
			}
			ErrorWrapper::OTHER(err) => {
				io::Error::new(ErrorKind::Other, err)
			}
		}
	}
}