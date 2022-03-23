use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, Result};
use base32::Alphabet;
use futures::StreamExt;
use hex::encode_to_slice;
use tokio::io::AsyncReadExt;
use tokio::task::block_in_place;
use zip::{CompressionMethod, DateTime, ZipWriter};
use zip::write::FileOptions;

use crate::file_scanner::scan_recursive;

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

		let mut file = File::create(&target_path)?;
		let mut zip = ZipWriter::new(file);

		let mut buf = [0u8; 4096];
		while let Some(file) = scans.next().await {
			if let Ok(file) = file {
				zip.start_file(file.file_name().to_string_lossy(), FileOptions::default().compression_method(CompressionMethod::Deflated));
				let mut rfile = tokio::fs::File::open(file.path()).await?;

				loop {
					let len = rfile.read(&mut buf).await?;
					if len == 0 { break; }
					block_in_place(|| {
						zip.write_all(&buf)
					})?;
				}
			}
		}
		let mut file = block_in_place(|| {
			zip.finish()
		})?;
		block_in_place(|| {
			file.flush()
		})?;
		Ok(target_path)
	}
}