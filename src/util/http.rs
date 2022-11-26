use std::io::Result;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures::StreamExt;
use reqwest::{Client, ClientBuilder};
use reqwest::header::{CONTENT_DISPOSITION, HeaderValue};
use reqwest::redirect::Policy;
use sha2::{Digest, Sha256};
use tokio::fs::metadata;
use tokio::io::AsyncWriteExt;
use tracing::{debug, trace};

use crate::util::errors::reqwest_to_io;
use crate::util::fs::new_file;

// return (DownloadedLocation, Sha256)
pub async fn download_to(client: &Client, url: &str, target: impl AsRef<Path>) -> Result<(PathBuf, String)> {
	let target = target.as_ref();
	debug!("downloading {url:?} to {target:?}");
	let resp = client.get(url).send().await.map_err(reqwest_to_io)?;
	let headers = resp.headers();
	let mut sha = Sha256::default();
	trace!("Getting filename..");
	let file_name = headers.get(CONTENT_DISPOSITION)
		.and_then(|it: &HeaderValue| {
			let bytes = it.as_bytes();
			let idx = bytes.windows(9)
				.position(|window| window == b"filename=")?;
			let (_, data) = bytes.split_at(idx + 9);
			Some(data)
		})
		.map(|it: &[u8]| String::from_utf8_lossy(it).trim_matches(['"', ' '].as_slice()).to_string());
	let meta = metadata(target).await;
	let out = if meta.is_ok() && meta.as_ref().unwrap().is_dir() {
		if let Some(name) = file_name {
			target.join(name)
		} else {
			PathBuf::from(target)
		}
	} else {
		PathBuf::from(target)
	};
	let mut file = new_file(&out).await.expect("Create file");
	let mut data = resp.bytes_stream();
	let mut file_len = 0u64;
	while let Some(data) = data.next().await {
		let bytes: Bytes = data.map_err(reqwest_to_io)?;
		file.write_all(&bytes).await?;
		file_len = file_len + (bytes.len() as u64);
		sha = tokio_rayon::spawn(|| {
			sha.update(bytes);
			sha
		}).await;
	}
	debug!("content from {url} has been downloaded to {out:?}");
	file.flush().await?;
	// due replace existing file truncate its size
	if meta?.len() != file_len {
		file.set_len(file_len).await?;
	}
	file.shutdown().await?;
	let hash = tokio_rayon::spawn(move || {
		String::from_iter(sha.finalize()[..].iter().map(|it| format!("{:02x}", it)))
	}).await;
	Ok((out, hash))
}

pub fn new_client() -> Result<Client> {
	ClientBuilder::new()
		.user_agent("curl/7.86.0")
		.redirect(Policy::limited(5))
		.build()
		.map_err(reqwest_to_io)
}