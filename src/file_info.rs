use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;
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
	let mut context = Sha256::default();
	let mut buffer = vec![0u8; 4096];

	// read up to 10 bytes
	loop {
		match file.read(&mut buffer).await {
			Ok(0) => {
				break;
			}
			Ok(n) => {
				let (ctx, buf) = tokio_rayon::spawn(move || {
					context.update(&buffer[..n]);
					(context, buffer)
				}).await;
				context = ctx;
				buffer = buf;
			}
			Err(e) => {
				Err(e)?
			}
		};
	}
	Ok(context.finalize()[..].iter().map(|x| format!("{:02x}", x)).collect::<String>())
}