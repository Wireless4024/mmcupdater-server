use std::path::PathBuf;

use anyhow::Result;
use md5::Context;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
	pub file: String,
	pub hash: String,
}

impl FileInfo {
	pub async fn new(path: PathBuf) -> Result<Self> {
		let hash = hash_file(File::open(&path).await?).await?;
		Ok(Self {
			file: path.file_name().unwrap().to_string_lossy().to_string(),
			hash,
		})
	}
}

pub async fn hash_file(mut file: File) -> Result<String> {
	let mut context = Context::new();
	let mut buffer = [0u8; 4096];

	// read up to 10 bytes
	loop {
		match file.read(&mut buffer).await {
			Ok(0) => {
				break;
			}
			Ok(n) => {
				context.consume(&buffer[..n]);
			}
			Err(e) => {
				Err(e)?
			}
		};
	}
	Ok(context.compute().0.iter().map(|x| format!("{:02x}", x)).collect::<String>())
}