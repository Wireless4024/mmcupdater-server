use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use base32::Alphabet;
use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zip::{CompressionMethod, ZipWriter};
use zip::write::FileOptions;

use crate::file_scanner::scan_recursive;

pub mod http;
pub mod errors;
pub mod fs;
pub mod java;
pub mod gh;
pub mod platform;
pub mod process;
pub mod logger;
pub mod config;
pub mod safe_writer;
pub mod time;
pub mod modification;
pub mod serde;
pub mod string;

pub async fn get_zip_file(path: PathBuf) -> Result<PathBuf> {
	if path.is_file() {
		Ok(path)
	} else {
		//let  output = path.parent().unwrap().join()
		let mut target = base32::encode(Alphabet::Crockford, path.to_string_lossy().as_bytes());
		target.reserve_exact(4);
		target.push_str(".zip");

		let target_path = path.parent().unwrap().join(target);
		/*if target_path.exists() {
			return Ok(target_path);
		}*/

		let mut scans = scan_recursive(path);

		let file = File::create(&target_path)?;
		let mut zip = ZipWriter::new(file);

		let mut buf = vec![0u8; 4096];
		while let Some(file) = scans.next().await {
			if let Ok(file) = file {
				zip.start_file(file.file_name().to_string_lossy(),
				               FileOptions::default().compression_method(CompressionMethod::Deflated))?;
				let mut rfile = tokio::fs::File::open(file.path()).await?;

				loop {
					let len = rfile.read(&mut buf).await?;
					if len == 0 { break; }
					let res = tokio_rayon::spawn(move || {
						zip.write_all(&buf)?;
						Result::<(ZipWriter<_>, Vec<_>)>::Ok((zip, buf))
					}).await?;
					zip = res.0;
					buf = res.1;
				}
			}
		}
		let file = tokio_rayon::spawn(move || {
			zip.finish()
		}).await?;
		tokio::fs::File::from_std(file).flush().await?;
		Ok(target_path)
	}
}

pub fn sync_send<T: Send + Sync>(_:T) {}