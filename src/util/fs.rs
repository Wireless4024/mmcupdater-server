use std::ffi::OsString;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::digest::Digest;
use sha2::Sha256;
use tar::Archive;
use tokio::fs::{create_dir_all, File, metadata, OpenOptions, read_dir, rename};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::debug;
use zip::ZipArchive;

use crate::util::errors::zip_to_io;

/// Attempt to create new file and create its parent directory if needed
pub async fn new_file(path: impl AsRef<Path>) -> io::Result<File> {
	let path = path.as_ref();
	debug!("Creating new file {path:?}");
	let parent = match path.parent() {
		None => {
			return Err(io::Error::from(ErrorKind::InvalidFilename));
		}
		Some(p) => { p }
	};
	let meta = metadata(parent).await;
	if meta.is_err() {
		create_dir_all(parent).await?;
	}
	OpenOptions::new()
		.create(true)
		.append(false)
		.write(true)
		.open(path)
		.await
}

/// Replace file content and close
pub async fn write_file(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> io::Result<()> {
	let mut file = new_file(path).await?;
	file.write_all(content.as_ref()).await?;
	file.flush().await?;
	file.shutdown().await?;
	Ok(())
}

/// Check if file is existed or create new file with content
pub async fn create_if_not_existed(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> io::Result<bool> {
	let path = path.as_ref();
	if metadata(&path).await.is_ok() {
		return Ok(true);
	}
	let mut file = new_file(path).await?;
	file.write_all(content.as_ref()).await?;
	file.flush().await?;
	file.shutdown().await?;
	Ok(false)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OwnedDirEntry {
	pub is_dir: bool,
	pub name: OsString,
}

pub async fn sha256(path: impl AsRef<Path>) -> io::Result<String> {
	let mut f = File::open(path).await?;
	let mut sha = Sha256::default();
	let mut buf = vec![0u8; 4096];
	loop {
		let len = f.read(&mut buf).await?;
		if len == 0 {
			break;
		}
		let res = tokio_rayon::spawn(move || {
			sha.update(&buf[..len]);
			(sha, buf)
		}).await;
		sha = res.0;
		buf = res.1;
	}

	Ok(tokio_rayon::spawn(move || {
		String::from_iter(sha.finalize()[..].iter().map(|it| format!("{:02x}", it)))
	}).await)
}

pub async fn verify_hash(path: impl AsRef<Path>, hash: &str) -> io::Result<()> {
	if sha256(path).await?.as_str() == hash {
		Ok(())
	} else {
		Err(io::Error::new(ErrorKind::InvalidData, "Invalid file hash"))
	}
}

pub async fn extract_archive(path: impl AsRef<Path>, target: impl AsRef<Path>) -> io::Result<()> {
	let path = path.as_ref();
	let file = File::open(path).await?.into_std().await;
	let target = target.as_ref().to_owned();
	if metadata(&target).await.is_err() {
		create_dir_all(&target).await?;
	}
	if let Some(true) = path.file_name().map(|it| it.to_string_lossy().ends_with(".tar.gz")) {
		let target = target.clone();
		tokio_rayon::spawn(move || {
			let mut tar = Archive::new(GzDecoder::new(file));
			tar.unpack(target)
		}).await?;
	} else {
		let target = target.clone();
		tokio_rayon::spawn(move || {
			let mut zip = ZipArchive::new(file)?;
			zip.extract(target)
		}).await.map_err(zip_to_io)?;
	}
	handle_archive(target).await?;
	Ok(())
}

// normalize file into folder here
pub async fn handle_archive(path: PathBuf) -> io::Result<()> {
	let mut read_dir = read_dir(&path).await?;
	let dir = read_dir.next_entry().await?;
	if dir.is_some() {
		// directory only have one directory
		if read_dir.next_entry().await?.is_none() {
			if let Some(parent) = path.parent() {
				let d = dir.unwrap().path();
				let temp = parent.join(d.file_name().unwrap());
				rename(d, &temp).await?;
				rename(temp, path).await?;
			}
		}
	}
	Ok(())
}