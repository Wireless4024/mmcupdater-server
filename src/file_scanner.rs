use std::future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use base32::Alphabet;
use futures::{stream, Stream, StreamExt};
use futures::future::{join_all, ok};
use md5::Context;
use regex::internal::Input;
use regex::Regex;
use tokio::{fs, io};
use tokio::fs::{DirEntry, File, read_dir};
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use tokio::task::spawn_local;

use crate::file_info::FileInfo;

pub async fn scan_files_exclude(path: impl AsRef<Path>, exclude: impl AsRef<str>) -> Result<Vec<PathBuf>> {
	let reg = Regex::new(exclude.as_ref())?;
	scan_files(path, |it| !reg.is_match(it.file_name().to_string_lossy().as_ref())).await
}

pub async fn scan_files<F>(path: impl AsRef<Path>, filter: F) -> Result<Vec<PathBuf>>
	where F: Fn(&DirEntry) -> bool {
	let mut dir = read_dir(path).await?;
	let mut files = Vec::new();
	while let Some(ent) = dir.next_entry().await? {
		if ent.metadata().await?.is_file() && filter(&ent) {
			files.push(ent.path());
		}
	}
	Ok(files)
}